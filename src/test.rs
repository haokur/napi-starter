use jieba_rs::Jieba;
use napi::Error;
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::tokenizer::*;
use tantivy::{doc, Index};

#[derive(Clone)]
struct JiebaTokenizer {
  jieba: Jieba,
}

struct JiebaTokenStream {
  tokens: Vec<Token>,
  index: usize,
}

impl TokenStream for JiebaTokenStream {
  fn advance(&mut self) -> bool {
    self.index += 1;
    self.index < self.tokens.len()
  }

  fn token(&self) -> &Token {
    &self.tokens[self.index]
  }

  fn token_mut(&mut self) -> &mut Token {
    &mut self.tokens[self.index]
  }
}

impl Tokenizer for JiebaTokenizer {
  type TokenStream<'a>
    = JiebaTokenStream
  where
    Self: 'a;

  fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
    let tokens: Vec<Token> = self
      .jieba
      .tokenize(text, jieba_rs::TokenizeMode::Default, true)
      .into_iter()
      .enumerate()
      .map(|(i, word)| Token {
        offset_from: word.start,
        offset_to: word.end,
        position: i,
        text: word.word.to_string(),
        position_length: 1,
      })
      .collect();
    JiebaTokenStream { tokens, index: 0 }
  }
}

fn setup_index(index_path: &str) -> tantivy::Result<(Index, Field, Field, Schema)> {
  // 清理旧索引
  // let index_path = "./tantivy_index";
  // if std::path::Path::new(index_path).exists() {
  //   std::fs::remove_dir_all(index_path)?;
  // }

  // 初始化分词器
  let jieba = Jieba::new();
  let mut tokenizer = JiebaTokenizer {
    jieba: jieba.clone(),
  };

  // 测试分词
  println!("--- Tokenizer Test ---");
  let text = "中文搜索测试";
  let mut token_stream = tokenizer.token_stream(text);
  while token_stream.advance() {
    println!("Token: {:?}", token_stream.token());
  }

  // 创建Schema
  let mut schema_builder = SchemaBuilder::new();
  let text_opts = TextOptions::default().set_stored().set_indexing_options(
    TextFieldIndexing::default()
      .set_tokenizer("jieba")
      .set_index_option(IndexRecordOption::WithFreqsAndPositions),
  );

  let title = schema_builder.add_text_field("title", text_opts.clone());
  let body = schema_builder.add_text_field("body", text_opts);
  let schema = schema_builder.build();

  // 创建新索引

  // let index = Index::create_in_dir(index_path, schema.clone())?;
  let path = std::path::Path::new(&index_path);
  // 判断是否已存在 index
  let index = if path.exists() {
    let existing = Index::open_in_dir(&path)
      .map_err(|e| Error::from_reason(format!("打开索引失败: {}", e)))
      .unwrap();
    existing
  } else {
    Index::create_in_dir(&path, schema.clone())
      .map_err(|e| Error::from_reason(format!("创建索引失败: {}", e)))
      .unwrap()
  };

  index.tokenizers().register(
    "jieba",
    JiebaTokenizer {
      jieba: jieba.clone(),
    },
  );

  Ok((index, title, body, schema))
}

fn write_index(index_path: &str, title_data: &str, body_data: &str) -> tantivy::Result<()> {
  let (index, title, body, _) = setup_index(index_path)?;

  let mut writer = index.writer(50_000_000)?;
  writer.add_document(doc!(
      title => title_data,
      body => body_data
  ))?;

  writer.commit()?;

  Ok(())
}

fn search_index(index_path: &str) -> tantivy::Result<Vec<String>> {
  let (index, title, body, schema) = setup_index(index_path)?;

  // 搜索
  let reader = index.reader()?;
  let searcher = reader.searcher();
  println!("Total docs: {}", searcher.num_docs());

  // 使用TermQuery对比
  let term = Term::from_field_text(body, "分词");
  let term_query = TermQuery::new(term, IndexRecordOption::Basic);
  let term_top_docs = searcher.search(&term_query, &TopDocs::with_limit(10))?;
  println!("Found {} docs with TermQuery", term_top_docs.len());

  let mut results = Vec::new();
  for (_score, doc_address) in term_top_docs {
    let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address).unwrap();
    let match_item = retrieved_doc.to_json(&schema);
    results.push(match_item);
    println!("{}", retrieved_doc.to_json(&schema));
  }
  Ok(results)
}

#[test]
fn search_index5() -> tantivy::Result<()> {
  // 清理旧索引
  let index_path = "./tantivy_index";
  // if std::path::Path::new(index_path).exists() {
  //   std::fs::remove_dir_all(index_path)?;
  // }
  let (index, title, body, schema) = setup_index(index_path)?;

  let result = search_index(index_path);
  println!("{:?}", result);

  // write_index(index_path, "中文搜索测试", "中文搜索测试的body内容区域")
  //   .expect("TODO: panic message");
  // write_index(index_path, "中文搜索测试", "这是一个用Rust和Tantivy实现的全文搜索引擎，支持中文分词。")
  //     .expect("TODO: panic message");
  // // 初始化分词器
  // let jieba = Jieba::new();
  // let mut tokenizer = JiebaTokenizer {
  //   jieba: jieba.clone(),
  // };
  //
  // // 测试分词
  // println!("--- Tokenizer Test ---");
  // let text = "中文搜索测试";
  // let mut token_stream = tokenizer.token_stream(text);
  // while token_stream.advance() {
  //   println!("Token: {:?}", token_stream.token());
  // }
  //
  // // 创建Schema
  // let mut schema_builder = SchemaBuilder::new();
  // let text_opts = TextOptions::default().set_stored().set_indexing_options(
  //   TextFieldIndexing::default()
  //     .set_tokenizer("jieba")
  //     .set_index_option(IndexRecordOption::WithFreqsAndPositions),
  // );
  //
  // let title = schema_builder.add_text_field("title", text_opts.clone());
  // let body = schema_builder.add_text_field("body", text_opts);
  // let schema = schema_builder.build();
  //
  // // 创建新索引
  //
  // // let index = Index::create_in_dir(index_path, schema.clone())?;
  // let path = std::path::Path::new(&index_path);
  // // 判断是否已存在 index
  // let index = if path.exists() {
  //   let existing = Index::open_in_dir(&path)
  //     .map_err(|e| Error::from_reason(format!("打开索引失败: {}", e)))
  //     .unwrap();
  //   existing
  // } else {
  //   Index::create_in_dir(&path, schema.clone())
  //     .map_err(|e| Error::from_reason(format!("创建索引失败: {}", e)))
  //     .unwrap()
  // };
  //
  // index.tokenizers().register(
  //   "jieba",
  //   JiebaTokenizer {
  //     jieba: jieba.clone(),
  //   },
  // );

  // 写入测试数据
  // let mut writer = index.writer(50_000_000)?;
  // writer.add_document(doc!(
  //     title => "中文搜索测试",
  //     body => "这是一个用Rust和Tantivy实现的全文搜索引擎，支持中文分词。"
  // ))?;
  // writer.add_document(doc!(
  //     title => "Rust开发",
  //     body => "Rust是一种系统编程语言，安全且高性能，适合构建搜索引擎。"
  // ))?;
  // writer.commit()?;

  // 搜索
  // let reader = index.reader()?;
  // let searcher = reader.searcher();
  // println!("Total docs: {}", searcher.num_docs());

  // // 创建QueryParser（关键修正点）
  // let query_parser = QueryParser::for_index(&index, vec![title, body]);
  // println!("Parser fields: {:?}", query_parser.parse_query("中文")); // 检查字段
  //
  // // 测试查询
  // let query = query_parser.parse_query("\"中文\"")?;
  // println!("Parsed query: {:?}", query);
  //
  // let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
  // println!("Found {} docs with QueryParser", top_docs.len());

  // 使用TermQuery对比
  // let term = Term::from_field_text(body, "分词");
  // let term_query = TermQuery::new(term, IndexRecordOption::Basic);
  // let term_top_docs = searcher.search(&term_query, &TopDocs::with_limit(10))?;
  // println!("Found {} docs with TermQuery", term_top_docs.len());
  //
  // for (_score, doc_address) in term_top_docs {
  //   let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address).unwrap();
  //   let match_item = retrieved_doc.to_json(&schema);
  //   // results.push(match_item);
  //   println!("{}", retrieved_doc.to_json(&schema));
  // }
  Ok(())
}
