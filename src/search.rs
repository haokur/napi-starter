use napi::bindgen_prelude::*;
use napi_derive::napi;
use tantivy::{
  collector::TopDocs,
  doc,
  query::QueryParser,
  schema::{Schema, SchemaBuilder, STORED, TEXT},
  Document as TantivyDocument, Index,
};

/// 创建 schema，只在新建索引时使用
fn build_schema() -> Schema {
  let mut builder = SchemaBuilder::new();
  builder.add_text_field("title", TEXT | STORED);
  builder.add_text_field("body", TEXT | STORED);
  builder.build()
}

/// 写入文档到 index
#[napi]
pub fn write_index(index_path: String, title: String, body: String) -> napi::Result<()> {
  let path = std::path::Path::new(&index_path);
  let schema;

  // 判断是否已存在 index
  let index = if path.exists() {
    let existing =
      Index::open_in_dir(&path).map_err(|e| Error::from_reason(format!("打开索引失败: {}", e)))?;
    schema = existing.schema();
    existing
  } else {
    schema = build_schema();
    Index::create_in_dir(&path, schema.clone())
      .map_err(|e| Error::from_reason(format!("创建索引失败: {}", e)))?
  };

  let mut writer = index
    .writer(50_000_000)
    .map_err(|e| Error::from_reason(e.to_string()))?;

  let title_field = schema.get_field("title").unwrap();
  let body_field = schema.get_field("body").unwrap();

  writer
    .add_document(doc!(title_field => title, body_field => body))
    .map_err(|e| Error::from_reason(e.to_string()))?;

  writer
    .commit()
    .map_err(|e| Error::from_reason(e.to_string()))?;
  Ok(())
}

/// 查询索引
#[napi]
pub fn search_index(index_path: String, query_str: String) -> napi::Result<Vec<String>> {
  let index = Index::open_in_dir(index_path).map_err(|e| Error::from_reason(e.to_string()))?;
  let schema = index.schema();

  let reader = index
    .reader()
    .map_err(|e| Error::from_reason(e.to_string()))?;
  reader
    .reload()
    .map_err(|e| Error::from_reason(e.to_string()))?;
  let searcher = reader.searcher();

  let default_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();

  let query_parser = QueryParser::for_index(&index, default_fields);
  let query = query_parser
    .parse_query(&query_str)
    .map_err(|e| Error::from_reason(e.to_string()))?;

  let top_docs = searcher
    .search(&query, &TopDocs::with_limit(10))
    .map_err(|e| Error::from_reason(e.to_string()))?;

  let mut results = Vec::new();

  for (_score, doc_address) in top_docs {
    let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address).unwrap();
    let match_item = retrieved_doc.to_json(&schema);
    results.push(match_item);
    println!("{}", retrieved_doc.to_json(&schema));
  }

  Ok(results)
}

#[test]
fn test_write() {
  // write_index(
  //   "./tantivy_index".to_owned(),
  //   "The Old Man and the Sea".to_owned(),
  //   "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone \
  //        eighty-four days now without taking a fish."
  //     .to_owned(),
  // )
  // .expect("TODO: panic message");

  write_index(
    "./tantivy_index".to_owned(),
    "中文搜索试试".to_owned(),
    "这是一段很长很长的中文啊."
        .to_owned(),
  )
      .expect("TODO: panic message");
}

#[test]
fn test_search() {
  // let result = search_index("./tantivy_index".to_owned(), "Sea".to_owned());
  // println!("{:#?}", result);


  let result = search_index("./tantivy_index".to_owned(), "fished".to_owned());
  println!("{:#?}", result);
}
