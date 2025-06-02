use jieba_rs::Jieba;
use napi::Error;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
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
    let tokens = self
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

/// 创建 schema 并返回字段引用
fn create_schema() -> (Schema, Field, Field) {
  let mut schema_builder = SchemaBuilder::new();
  let text_options = TextOptions::default().set_stored().set_indexing_options(
    TextFieldIndexing::default()
      .set_tokenizer("jieba")
      .set_index_option(IndexRecordOption::WithFreqsAndPositions),
  );

  let title = schema_builder.add_text_field("title", text_options.clone());
  let body = schema_builder.add_text_field("body", text_options);
  let schema = schema_builder.build();

  (schema, title, body)
}

/// 初始化索引并注册分词器
fn init_index(index_path: &str, schema: &Schema) -> tantivy::Result<Index> {
  let path = std::path::Path::new(index_path);

  let index = if path.exists() {
    Index::open_in_dir(&path)?
  } else {
    Index::create_in_dir(&path, schema.clone())?
  };

  let jieba = Jieba::new();
  index
    .tokenizers()
    .register("jieba", JiebaTokenizer { jieba });

  Ok(index)
}

/// 完整设置索引和 schema、字段
fn setup_index(index_path: &str) -> tantivy::Result<(Index, Field, Field, Schema)> {
  let (schema, title, body) = create_schema();
  let index = init_index(index_path, &schema)?;
  Ok((index, title, body, schema))
}

/// 写入文档
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

/// 搜索文档
fn search_index(index_path: &str, query_str: &str) -> tantivy::Result<Vec<String>> {
  let (index, _, _, schema) = setup_index(index_path)?;

  let reader = index.reader()?;
  let searcher = reader.searcher();

  println!("Total docs: {}", searcher.num_docs());

  let fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();
  let query_parser = QueryParser::for_index(&index, fields);
  let query = query_parser.parse_query(query_str)?;
  println!("query: {:?}", query);

  let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

  let results = top_docs
    .into_iter()
    .map(|(_, addr)| {
      let doc: tantivy::TantivyDocument = searcher.doc(addr).unwrap();
      doc.to_json(&schema)
    })
    .collect();

  Ok(results)
}


#[test]
fn setup_index_test(){
  let index_path = "./tantivy_index";
  let mut builder = SchemaBuilder::new();
  builder.add_text_field("title", TEXT | STORED);
  builder.add_text_field("body", TEXT | STORED);
  let schema = builder.build();
  Index::create_in_dir(&index_path, schema).unwrap();
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
fn test_search() -> tantivy::Result<()> {
  let index_path = "./tantivy_index";
  // setup_index(index_path)?;

  // let result = search_index(index_path, "The Old Man and the Sea");
  // let result = search_index(index_path, "全文");
  let result = search_index(index_path, "全文解锁");

  println!("{:?}", result);

  Ok(())
}
