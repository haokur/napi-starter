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
  // 初始化分词器
  let jieba = Jieba::new();
  let mut tokenizer = JiebaTokenizer {
    jieba: jieba.clone(),
  };

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

fn search_index(index_path: &str, body_str: &str) -> tantivy::Result<Vec<String>> {
  let (index, title, body, schema) = setup_index(index_path)?;

  // 搜索
  let reader = index.reader()?;
  let searcher = reader.searcher();
  println!("Total docs: {}", searcher.num_docs());

  let default_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();
  let query_parser = QueryParser::for_index(&index, default_fields);
  let query = query_parser.parse_query(&body_str)?;
  let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
  let mut results = Vec::new();

  for (_score, doc_address) in top_docs {
    let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address).unwrap();
    let match_item = retrieved_doc.to_json(&schema);
    results.push(match_item);
  }
  Ok(results)
}

fn search_index2(index_path: &str, query_str: String) -> tantivy::Result<Vec<String>> {
  let (index, title, body, schema) = setup_index(index_path)?;
  let schema = index.schema();

  let reader = index
    .reader()
    .map_err(|e| Error::from_reason(e.to_string()))
    .unwrap();
  reader
    .reload()
    .map_err(|e| Error::from_reason(e.to_string()))
    .unwrap();

  let searcher = reader.searcher();
  let default_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();
  let query_parser = QueryParser::for_index(&index, default_fields);
  let query = query_parser
    .parse_query(&query_str)
    .map_err(|e| Error::from_reason(e.to_string()))
    .unwrap();

  let top_docs = searcher
    .search(&query, &TopDocs::with_limit(10))
    .map_err(|e| Error::from_reason(e.to_string()))
    .unwrap();

  let mut results = Vec::new();

  for (_score, doc_address) in top_docs {
    let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address).unwrap();
    let match_item = retrieved_doc.to_json(&schema);
    results.push(match_item);
  }

  Ok(results)
}

#[test]
fn test_write() {
  let index_path = "./tantivy_index";
  write_index(index_path, "全文解锁", "使用rust开发全文搜索功能").expect("TODO: panic message");

  write_index(
    index_path,
    "The Old Man and the Sea",
    "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone \
         eighty-four days now without taking a fish..",
  )
  .expect("TODO: panic message");
}

#[test]
fn clean_index() {
  let index_path = "./tantivy_index";
  if std::path::Path::new(index_path).exists() {
    std::fs::remove_dir_all(index_path).unwrap();
  }
}

#[test]
fn search_index5() -> tantivy::Result<()> {
  let index_path = "./tantivy_index";
  // setup_index(index_path)?;

  let result = search_index(index_path, "全文搜索功能");
  println!("{:?}", result);

  Ok(())
}
