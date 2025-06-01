use napi::bindgen_prelude::*;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Document, SchemaBuilder, STORED, TEXT};
use tantivy::{doc, Index, TantivyDocument};

#[test]
fn test_search() {
  let result = search_index(String::from("./tantivy_index"), String::from("搜索"));
  println!("result is {:?}", result);
}

#[napi]
pub fn search_index(index_path: String, query_str: String) -> napi::Result<Vec<String>> {
  let index = Index::open_in_dir(index_path).map_err(|e| Error::from_reason(e.to_string()))?;

  let reader = index.reader().map_err(|e| Error::from_reason(e.to_string()))?;
  reader.reload().map_err(|e| Error::from_reason(e.to_string()))?; // ✅ 关键

  let searcher = reader.searcher();
  let schema = index.schema();
  println!("schema: {:?}", schema);
  let default_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();
  println!("default_fields: {:?}", default_fields);
  let query_parser = QueryParser::for_index(&index, default_fields);
  let query = query_parser
      .parse_query(&query_str)
      .map_err(|e| Error::from_reason(e.to_string()))?;

  let top_docs = searcher
      .search(&query, &TopDocs::with_limit(10))
      .map_err(|e| Error::from_reason(e.to_string()))?;

  let mut results = Vec::new();
  for (_score, doc_address) in top_docs {
    let compact_doc = searcher
        .doc(doc_address)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let doc = TantivyDocument::from(compact_doc);
    let json = doc.to_json(&schema);
    results.push(json);
  }

  Ok(results)
}


// #[napi]
// pub fn search_index(index_path: String, query_str: String) -> napi::Result<Vec<String>> {
//   let index = Index::open_in_dir(index_path).map_err(|e| Error::from_reason(e.to_string()))?;
//
//   let reader = index
//     .reader()
//     .map_err(|e| Error::from_reason(e.to_string()))?;
//   reader
//     .reload()
//     .map_err(|e| Error::from_reason(e.to_string()))?; // 👈 关键一步
//   let searcher = reader.searcher();
//
//   let schema = index.schema();
//   let default_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();
//
//   let query_parser = QueryParser::for_index(&index, default_fields);
//   let query = query_parser
//     .parse_query(&query_str)
//     .map_err(|e| Error::from_reason(e.to_string()))?;
//
//   let top_docs = searcher
//     .search(&query, &TopDocs::with_limit(10))
//     .map_err(|e| Error::from_reason(e.to_string()))?;
//
//   println!("top_docs: {:?}", top_docs);
//
//   let mut results = Vec::new();
//   for (_score, doc_address) in top_docs {
//     let compact_doc = searcher
//       .doc(doc_address)
//       .map_err(|e| Error::from_reason(e.to_string()))?;
//
//     let doc = TantivyDocument::from(compact_doc);
//     let json = doc.to_json(&schema);
//     results.push(json);
//   }
//
//   Ok(results)
// }

// #[napi]
// pub fn search_index(index_path: String, query_str: String) -> napi::Result<Vec<String>> {
//   let index = Index::open_in_dir(index_path).map_err(|e| Error::from_reason(e.to_string()))?;
//
//   let reader = index
//     .reader()
//     .map_err(|e| Error::from_reason(e.to_string()))?;
//   let searcher = reader.searcher();
//   let schema = index.schema();
//   let default_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();
//
//   let query_parser = QueryParser::for_index(&index, default_fields);
//   let query = query_parser
//     .parse_query(&query_str)
//     .map_err(|e| Error::from_reason(e.to_string()))?;
//
//   let top_docs = searcher
//     .search(&query, &TopDocs::with_limit(10))
//     .map_err(|e| Error::from_reason(e.to_string()))?;
//
//   let mut results = Vec::new();
//   for (_score, doc_address) in top_docs {
//     let compact_doc = searcher
//       .doc(doc_address)
//       .map_err(|e| Error::from_reason(e.to_string()))?;
//
//     let doc = TantivyDocument::from(compact_doc); // 👈 转为 Document
//     let json = doc.to_json(&schema); // 👈 传入 schema
//
//     results.push(json);
//   }
//
//   Ok(results)
// }

fn create_index(index_path: &str) -> tantivy::Result<()> {
  let mut schema_builder = SchemaBuilder::new();
  schema_builder.add_text_field("title", TEXT | STORED);
  schema_builder.add_text_field("body", TEXT | STORED);
  let schema = schema_builder.build();

  let index = Index::create_in_dir(index_path, schema)?;
  let mut index_writer = index.writer(50_000_000)?;

  let title = index.schema().get_field("title").unwrap();
  let body = index.schema().get_field("body").unwrap();

  index_writer.add_document(doc!(
      title => "Tantivy 中文搜索",
      body => "这是一个基于 Rust 的全文本搜索引擎"
  ))?;

  index_writer.commit()?;
  Ok(())
}

#[test]
fn test_create_index() {
  create_index("./tantivy_index").unwrap();
}

#[napi]
pub fn write_index(index_path: String, title_str: String, body_str: String) -> napi::Result<()> {
  // 创建 schema（或从已存在的 index 中读取）
  let schema = if std::path::Path::new(&index_path).exists() {
    let index = Index::open_in_dir(&index_path).map_err(|e| Error::from_reason(e.to_string()))?;
    index.schema()
  } else {
    let mut builder = SchemaBuilder::new();
    builder.add_text_field("title", TEXT | STORED);
    builder.add_text_field("body", TEXT | STORED);
    builder.build()
  };

  // 打开或创建 Index
  let index = if std::path::Path::new(&index_path).exists() {
    Index::open_in_dir(&index_path).map_err(|e| Error::from_reason(e.to_string()))?
  } else {
    Index::create_in_dir(&index_path, schema.clone())
      .map_err(|e| Error::from_reason(e.to_string()))?
  };

  // let title = schema.get_field("title").ok_or_else(|| Error::from_reason("Missing 'title' field"))?;
  // let body = schema.get_field("body").ok_or_else(|| Error::from_reason("Missing 'body' field"))?;
  let title = schema.get_field("title").unwrap();
  let body = schema.get_field("body").unwrap();

  let mut writer = index
    .writer(50_000_000)
    .map_err(|e| Error::from_reason(e.to_string()))?;

  writer.add_document(doc!(
    title => title_str,
    body => body_str
  ));

  writer
    .commit()
    .map_err(|e| Error::from_reason(e.to_string()))?;

  Ok(())
}

#[test]
fn test_write_index() {
  let result = write_index(
    String::from("./tantivy_index"),
    String::from("搜索测试"),
    String::from("搜索内容文字"),
  );
  println!("result is {:?}", result);

  let result = search_index(String::from("./tantivy_index"), String::from("搜索"));
  println!("result is {:?}", result);
}
