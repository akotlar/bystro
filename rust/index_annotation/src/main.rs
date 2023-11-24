// Add this content to main.rs
use crossbeam::channel::bounded;
use crossbeam::thread as cthread;
// use rkyv::{Archive, Deserialize, Serialize};
use serde::Serialize;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fs::File;
use std::io;

use std::io::{BufRead, BufReader};

const NEWLINE_DELIMITER: u8 = b'\n';
const POSITION_DELIMITER: u8 = b'|';
const EMPTY_FIELD_CHAR: &[u8; 2] = b"NA";
const VALUE_DELIMITER: &[u8; 1] = b";";
const FIELD_SEPARATOR: &[u8; 1] = b"\t";
const OVERLAP_DELIMITER: &[u8; 1] = b"/";
// use bumpalo::{collections::Vec, Bump};

type OverlapValue = SmallVec<[ParsedValue; 64]>;
type ValueValue = SmallVec<[OverlapValue; 1]>;
type PositionValue = SmallVec<[ValueValue; 1]>;

// Adjust the type alias as per your requirement
// type InnerArray<'a> = TinyVec<[&'a str; 5]>; // Change 3 to the max size of inner arrays
// type OuterArray<'a> = TinyVec<[InnerArray; 3]>; // Change 2 to the max size of outer arrays

#[derive(Debug, Serialize)]
enum NestedValue {
    Leaf(PositionValue),
    Node(HashMap<String, NestedValue>),
}

// fn convert_to_nested_map(flat_map: HashMap<&str, PositionValue>) -> HashMap<String, NestedValue> {
//     let mut nested_map: HashMap<String, NestedValue> = HashMap::new();

//     for (key, value) in flat_map {
//         insert_into_nested_map(&mut nested_map, key.split('.').collect(), value);
//     }

//     nested_map
// }

fn insert_into_nested_map(
    map: &mut HashMap<String, NestedValue>,
    mut parts: Vec<&str>,
    value: PositionValue,
) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        map.insert(parts[0].to_string(), NestedValue::Leaf(value));
        return;
    }

    let first_part = parts.remove(0);
    if !map.contains_key(first_part) {
        map.insert(first_part.to_string(), NestedValue::Node(HashMap::new()));
    }

    match map.get_mut(first_part).unwrap() {
        NestedValue::Node(sub_map) => insert_into_nested_map(sub_map, parts, value),
        _ => panic!("Expected a node, found a leaf"),
    }
}

fn parse_string(input: &[u8]) -> ParsedValue {
    if input == b"NA" {
        return ParsedValue::None;
    }

    let input = std::str::from_utf8(input).unwrap();
    if let Ok(float_value) = input.parse::<f64>() {
        ParsedValue::Float(float_value)
    } else {
        ParsedValue::String(input.to_string())
    }
}

#[derive(Debug, Serialize, Archive, Clone)]
enum ParsedValue {
    Float(f64),
    String(String),
    None,
}

impl Default for ParsedValue {
    fn default() -> Self {
        ParsedValue::None
    }
}

struct AnnotationReader {
    decompressed_data: BufReader<std::process::ChildStdout>,
    field_separator: char,
    position_delimiter: char,
    value_delimiter: char,
    overlap_delimiter: char,
    empty_field_char: char,
    index_name: String,
    id: usize,
    chunk_size: usize,
    // ... other fields ...
}

// fn bytes_to_json_value(bytes: &[u8]) -> Result<Value, Error> {
//     let s = std::str::from_utf8(bytes).unwrap();
//     serde_json::from_str(&s)
// }

#[allow(clippy::cognitive_complexity)]
fn process_lines(r: crossbeam::channel::Receiver<Vec<u8>>, header: &str) {
    let header_fields: Vec<&str> = header.split('\t').collect();

    // let mut bytes = Vec::new();
    // let mut count = 0;
    // let mut count = 0;

    // for _ in 0..100000 {
    //     let mut map: HashMap<String, NestedValue> = HashMap::with_capacity(header_fields.len());
    //     hash_maps.push(map);
    // }

    // let mut pool: Vec<SmallVec<[SmallVec<[SmallVec<[ParsedValue; 64]>; 1]>; 1]>> = Vec::new();

    // Pre-allocate a number of 3D structures
    // for _ in 0..1000 {
    //     // Assuming you need 100 pre-allocated structures
    //     let innermost = SmallVec::from_elem(ParsedValue::None, 64); // Pre-allocate with default values
    //     let middle = SmallVec::from_buf([innermost]);
    //     let outer = SmallVec::from_buf([middle]);
    //     pool.push(outer);
    // }

    // let mut hash_map: HashMap<String, NestedValue> = HashMap::with_capacity(1_000);
    let mut ops: Vec<BulkOperation<Value>> = Vec::with_capacity(500);

    loop {
        match r.recv() {
            Err(_) => break,
            Ok(message) => {
                // let mut arena = Bump::new();
                // let mut records: Vec<HashMap<String, NestedValue>> =
                // arena.alloc(Vec::with_capacity(20_000));
                // *arena.alloc(HashMap::<String, NestedValue>::with_capacity(10_000));

                // records.clear();
                // // let mut row_documents = Vec::new();

                for (_index, row) in message.split(|byt| *byt == NEWLINE_DELIMITER).enumerate() {
                    // println!("ROW: {:?}", index);
                    // let record_map = &mut hash_maps[index];
                    // record_map.clear();

                    // count += 1;
                    // println!("ROW: {:?}", String::from_utf8(row.to_vec()));

                    row_document.clear();
                    'field_loop: for (field_idx, field) in
                        row.split(|byt| *byt == FIELD_SEPARATOR).enumerate()
                    {
                        if field == b"NA" {
                            continue 'field_loop;
                        }

                        let has_pos_delim = field.contains(&POSITION_DELIMITER);
                        let has_val_delim = field.contains(&VALUE_DELIMITER);
                        let has_overlap_delim = field.contains(&OVERLAP_DELIMITER);

                        let values: PositionValue;
                        if !has_pos_delim && !has_val_delim && !has_overlap_delim {
                            let innermost = OverlapValue::from_elem(parse_string(field), 64); // Pre-allocate with default values
                            let _middle = ValueValue::from_buf([innermost]);
                            values = PositionValue::from_buf([_middle]);
                        } else if !has_pos_delim && !has_overlap_delim {
                            values = PositionValue::from_buf([field
                                .split(|&x| x == VALUE_DELIMITER)
                                .map(|value| OverlapValue::from_elem(parse_string(value), 64))
                                .collect()]);
                        } else if !has_val_delim && !has_overlap_delim {
                            values = field
                                .split(|&x| x == POSITION_DELIMITER)
                                .map(|value| {
                                    let innermost =
                                        OverlapValue::from_elem(parse_string(value), 64); // Pre-allocate with default values
                                    ValueValue::from_buf([innermost])
                                })
                                .collect();
                        } else if !has_val_delim && !has_pos_delim {
                            values = PositionValue::from_buf([ValueValue::from_buf([field
                                .split(|&x| x == OVERLAP_DELIMITER)
                                .map(|overlap_value| parse_string(overlap_value))
                                .collect()])]);
                        } else {
                            values = field
                                .split(|&x| x == POSITION_DELIMITER)
                                .map(|position_value| {
                                    position_value
                                        .split(|&x| x == VALUE_DELIMITER)
                                        .map(|value_value| {
                                            value_value
                                                .split(|&x| x == OVERLAP_DELIMITER)
                                                .map(|overlap_value| parse_string(overlap_value))
                                                .collect()
                                        })
                                        .collect()
                                })
                                .collect()
                        }

                        row_document.insert(header_fields[field_idx], values);

                        // insert_into_nested_map(
                        //     &mut hash_map,
                        //     header_fields_paths[field_idx].clone(),
                        //     values,
                        // );

                        // let mut position_values: PositionValue = SmallVec::new();
                        // for position_value in field.split(|&x| x == POSITION_DELIMITER) {
                        //     // if position_value == b"NA" {
                        //     //     position_values.push(ParsedValue::None);
                        //     //     continue;
                        //     // }

                        //     let mut value_values: ValueValue = SmallVec::new();
                        //     for value_value in position_value.split(|&x| x == b';') {
                        //         // if value_value == b"NA" {
                        //         //     value_values.push(ParsedValue::None);
                        //         //     continue;
                        //         // }

                        //         let mut overlap_values: OverlapValue = SmallVec::new();
                        //         for overlap_value in value_value.split(|&x| x == b'/') {
                        //             if overlap_value == b"NA" {
                        //                 overlap_values.push(ParsedValue::None);
                        //                 continue;
                        //             }

                        //             let s: ParsedValue =
                        //                 parse_string(std::str::from_utf8(overlap_value).unwrap());

                        //             overlap_values.push(s);
                        //         }

                        //         value_values.push(overlap_values);
                        //     }

                        //     position_values.push(value_values);
                        // }

                        // for (i, field_path) in header_fields_paths[idx].iter().enumerate() {
                        //     if i == header_fields_paths[idx].len() - 1 {
                        //         row_document.insert(header_fields_paths[idx][i], position_values);
                        //     } else {
                        //         row_document
                        //             .entry(field_path)
                        //             .or_insert_with(HashMap::new)
                        //             .downcast_mut::<HashMap<String, PositionValue>>()
                        //             .unwrap();
                        //     }
                    }

                    // row_document.insert(header_fields[idx], position_values);
                    // }

                    // convert_to_nested_map(row_document);
                    // let json = serde_json::to_string(&row_document).unwrap();
                    // println!("{}", json);
                    // println!("{:?}", convert_to_nested_map(row_document));

                    // count += 1;
                    // row_documents.push(json);

                    // if row_documents.len() >= 20_000 {
                    //     row_documents.clear();
                }
            } // }
        }
        // println!("Indexed: {:?}", count);
    }
}

// impl AnnotationReader {
//     // ... initialization and other methods ...

//     fn read_next_chunk(&mut self) -> Option<Vec<HashMap<String, String>>> {
//         let mut row_documents = Vec::new();

//         for line_result in self.decompressed_data.lines() {
//             let line = match line_result {
//                 Ok(ln) => ln,
//                 Err(_) => break,
//             };

//             let mut _source = HashMap::new();
//             let row: Vec<_> = line.split(self.FIELD_SEPARATOR).collect();

//             for (i, field) in row.iter().enumerate() {
//                 if *field == self.EMPTY_FIELD_CHAR.to_string() {
//                     continue;
//                 }

//                 let position_values: Vec<_> = field
//                     .split(self.POSITION_DELIMITER)
//                     .map(|pos_value| {
//                         if pos_value == self.EMPTY_FIELD_CHAR.to_string() {
//                             return None;
//                         }

//                         let values: Vec<_> = pos_value
//                             .split(self.VALUE_DELIMITER)
//                             .map(|value| {
//                                 if value == self.EMPTY_FIELD_CHAR.to_string() {
//                                     return None;
//                                 }

//                                 let overlap_values: Vec<_> = value
//                                     .split(self.OVERLAP_DELIMITER)
//                                     .filter_map(|overlap_value| {
//                                         if overlap_value == self.EMPTY_FIELD_CHAR.to_string() {
//                                             None
//                                         } else {
//                                             Some(overlap_value.to_string())
//                                         }
//                                     })
//                                     .collect();

//                                 Some(overlap_values)
//                             })
//                             .collect();

//                         Some(values)
//                     })
//                     .collect();

//                 // Populate _source (modify as per your requirements)
//                 _source.insert(i.to_string(), format!("{:?}", position_values));
//             }

//             if _source.is_empty() {
//                 continue;
//             }

//             self.id += 1;
//             row_documents.push(_source);

//             if row_documents.len() >= self.chunk_size {
//                 return Some(row_documents);
//             }
//         }

//         if row_documents.is_empty() {
//             None
//         } else {
//             Some(row_documents)
//         }
//     }
// }

// fn find_annotation_member(tar_path: &str, annotation_name: &str) -> Option<String> {
//     // Open the tar file
//     let file = File::open(tar_path).expect("Failed to open tar file");
//     let mut archive = Archive::new(file);

//     // Iterate through the members of the tar file
//     for file in archive.entries().expect("Failed to read tar entries") {
//         let file = file.expect("Failed to read tar entry");

//         // Check if the file name contains the annotation name
//         if let Some(path) = file.path().ok() {
//             if path
//                 .to_str()
//                 .map_or(false, |name| name.contains(annotation_name))
//             {
//                 return Some(path.to_string_lossy().into_owned());
//             }
//         }
//     }

//     None
// }

// struct ReadAnnotationTarball {
//     index_name: String,
//     chunk_size: usize,
//     // ... other fields ...
//     decompressed_data: BufReader<std::process::ChildStdout>,
// }

// impl ReadAnnotationTarball {
//     fn new(
//         index_name: &str,
//         tar_path: &str,
//         annotation_name: &str, /* other params */
//     ) -> ReadAnnotationTarball {
//         let annotation_file_name = find_annotation_member(tar_path, annotation_name)
//             .expect("Failed to find annotation file in tarball");

//         let command = format!(
//             "tar -xOf {} {} | bgzip --threads 8 -d -c",
//             tar_path, annotation_file_name
//         );

//         let child = Command::new("sh")
//             .arg("-c")
//             .arg(&command)
//             .stdout(Stdio::piped())
//             .spawn()
//             .expect("Failed to execute command");

//         let reader = BufReader::new(child.stdout.expect("Failed to open stdout"));

//         // for line in reader.lines().take(10) {
//         //     println!("Line: {}", line.expect("Failed to read line"));
//         // }

//         // ... initialization logic ...

//         ReadAnnotationTarball {
//             index_name: index_name.to_string(),
//             chunk_size: 1000,
//             // ... other fields ...
//             decompressed_data: reader,
//         }
//     }

//     // Other methods...
// }

// impl Iterator for ReadAnnotationTarball {
//     type Item = Vec<String>; // Adjust the type based on your requirements

//     // fn next(&mut self) -> Vec<String> {
//     //     println!("Reading next chunk",);
//     //     // Implementation of the iterator logic...
//     // }
// }

// fn main() {
//     let tar_path = "/seqant/user-data/63ddc9ce1e740e0020c39928/6555748df71022dc49c8e273/output/trio_trim_vep_vcf.tar";
//     let annotation_name = "annotation.tsv.gz";
//     let annotation_file_name = find_annotation_member(tar_path, annotation_name)
//         .expect("Failed to find annotation file in tarball");

//     let command = format!(
//         "tar -xOf {} {} | bgzip --threads 8 -d -c",
//         tar_path, annotation_file_name
//     );

//     let child = Command::new("sh")
//         .arg("-c")
//         .arg(&command)
//         .stdout(Stdio::piped())
//         .spawn()
//         .expect("Failed to execute command");

//     let reader = BufReader::new(child.stdout.expect("Failed to open stdout"));

//     let mut id = 0;
//     let chunk_size = 10000;
//     let mut row_documents = Vec::new();
//     // let reader = ReadAnnotationTarball::new("index_name", "/seqant/user-data/63ddc9ce1e740e0020c39928/6555748df71022dc49c8e273/output/trio_trim_vep_vcf.tar", "annotation.tsv.gz");
//     for record in reader.lines() {
//         // Process each record
//         // println!("Record: {:?}", record);
//         // {
//         let line = match record {
//             Ok(ln) => ln,
//             Err(_) => break,
//         };

//         let mut _source = HashMap::new();
//         let row: Vec<_> = line.split(FIELD_SEPARATOR).collect();

//         for (i, field) in row.iter().enumerate() {
//             if *field == EMPTY_FIELD_CHAR.to_string() {
//                 continue;
//             }

//             let position_values: Vec<Value> = field
//                 .split(POSITION_DELIMITER)
//                 .map(|pos_value| {
//                     if pos_value == EMPTY_FIELD_CHAR.to_string() {
//                         return serde_json::Value::Null;
//                     }

//                     let values: Vec<Value> = pos_value
//                         .split(VALUE_DELIMITER)
//                         .map(|value| {
//                             if value == EMPTY_FIELD_CHAR.to_string() {
//                                 return serde_json::Value::Null;
//                             }

//                             let overlap_values: Vec<Value> = value
//                                 .split(OVERLAP_DELIMITER)
//                                 .map(|overlap_value| {
//                                     if overlap_value == EMPTY_FIELD_CHAR.to_string() {
//                                         Value::Null
//                                     } else {
//                                         json!(overlap_value)
//                                     }
//                                 })
//                                 .collect();

//                             json!(overlap_values)

//                             // if overlap_values.iter().any(|v| matches!(v, Value::Null)) {
//                             //     Value::Null
//                             // } else {
//                             //     json!(overlap_values)
//                             // }

//                             // json!(overlap_values)
//                         })
//                         .collect();

//                     json!(values)
//                 })
//                 .collect();

//             // Populate _source with JSON values
//             _source.insert(i.to_string(), position_values);
//         }

//         if _source.is_empty() {
//             continue;
//         }

//         id += 1;
//         row_documents.push(_source);

//         // if id == 1 {
//         //     println!("{}", serde_json::to_string_pretty(&row_documents).unwrap());
//         // }

//         // println!("Row: {:?}", row_documents.len());
//     }

//     println!("Indexed: {:?}", row_documents.len());
//     // }
// }

fn get_header_and_num_eol_chars<T: BufRead>(reader: &mut T) -> (Vec<u8>, usize) {
    let mut buf = Vec::new();
    let len = reader.read_until(b'\n', &mut buf).unwrap();

    if len == 0 {
        panic!("Empty file")
    }

    // if !buf.starts_with(b"##fileformat=VCFv4") {
    //     panic!(
    //         "File format not supported: {}",
    //         std::str::from_utf8(&buf).unwrap()
    //     );
    // }

    let len = buf.len();

    if buf[len - 2] == b'\r' {
        return (buf[..len - 2].to_vec(), 2);
    }

    (buf[..len - 1].to_vec(), 1)
}

fn main() -> Result<(), io::Error> {
    let _n_worker_threads = num_cpus::get();

    let (head, _n_eol_chars) = get_header_and_num_eol_chars(&mut io::stdin().lock());

    let head = std::str::from_utf8(&head).unwrap();
    // header.write_output_header(io::stdout());
    // header.write_sample_list("sample-list.tsv");
    println!("{}", head);
    let (s1, r1) = bounded(1e3 as usize);
    // let n_samples = header.samples.len() as u32;
    cthread::scope(|scope| {
        scope.spawn(move |_| {
            let stdin = File::open("/dev/stdin").unwrap();
            // let size : File= stdin.metadata().unwrap().len() as usize;
            let mut reader = std::io::BufReader::with_capacity(256 * 1024 * 1024, stdin);
            let chunk_size = 256 * 1024 * 1024; // 200 MB
            let mut chunk = Vec::with_capacity(chunk_size);

            loop {
                let buffer = reader.fill_buf().unwrap();
                let buffer_len = buffer.len();

                if buffer_len == 0 {
                    break; // End of file
                }

                let end_with_newline = buffer[buffer.len() - 1] == b'\n';

                // Copy the buffer to the chunk
                chunk.extend_from_slice(buffer);
                // Consume the buffer
                reader.consume(buffer_len);

                // If the buffer does not end with a newline, continue reading byte-by-byte
                if !end_with_newline {
                    let mut buf = Vec::new();
                    let len = reader.read_until(b'\n', &mut buf).unwrap();

                    if len > 0 {
                        chunk.extend_from_slice(buf.as_slice());
                    }
                    // len = reader.read_until(b'\n', &mut buf).unwrap();
                    // let mut temp_buf = [0u8; 1];
                    // loop {
                    //     match reader.read(&mut temp_buf) {
                    //         Ok(0) | Err(_) => break, // End of stream or error
                    //         Ok(_) => {
                    //             let byte = temp_buf[0];
                    //             chunk.push(byte);

                    //             if byte == b'\n' {
                    //                 break;
                    //             }
                    //         }
                    //     }
                    // }
                }

                // Process the chunk here
                // ...
                // println!("CHUNK: {:?}", chunk.split(|byt| *byt == b'\n').count());
                s1.send(chunk.clone()).unwrap();

                // for line in chunk.split(|byt| *byt == b'\n') {
                //     // println!("LINE: {:?}", String::from_utf8(line.to_vec()))
                //     // lines.push(line.to_vec());
                //     // if lines.len() == max_lines {
                //     //     s1.send(lines).unwrap();
                //     //     lines = Vec::with_capacity(max_lines);
                //     // }
                // }
                // Clear the chunk for the next iteration
                chunk.clear();
            }

            // // Handle any remaining data in the chunk after the loop
            if !chunk.is_empty() {
                s1.send(chunk.clone()).unwrap();
                chunk.clear();
                //     // Process the remaining data in the chunk
                //     // ...
                //     println!("CHUNK: {:?}", chunk.split(|byt| *byt == b'\n').count());
            }

            // loop {
            //     let buffer = reader.fill_buf().unwrap();

            //     if buffer.is_empty() {
            //         break; // End of file
            //     }

            //     let mut end_with_newline = buffer[buffer.len() - 1] == b'\n';

            //     // Copy buffer to chunk or process it directly
            //     // ...

            //     if !end_with_newline {
            //         // Look for the newline character in the remaining data
            //         for byte in buffer.iter() {
            //             if *byte == b'\n' {
            //                 end_with_newline = true;
            //                 break;
            //             }
            //         }
            //     }

            //     let consumed = if end_with_newline {
            //         buffer.len()
            //     } else {
            //         buffer.len() - 1 // Leave the last byte for the next iteration
            //     };

            //     // Tell the reader how much we've used
            //     reader.consume(consumed);

            //     // Process the chunk or buffer
            //     // ...
            // }

            // let chunk_size = 200 * 1024 * 1024; // 200 MB
            // let mut chunk = vec![0; chunk_size];
            // loop {
            //     let buffer = reader.fill_buf().unwrap();
            //     // let mut len = reader.read(&mut chunk).unwrap();

            //     if buffer.len() == 0 {
            //         break; // End of file
            //     }

            //     // Ensure we end with a newline character
            //     if buffer[buffer.len() - 1] != b'\n' {
            //         let mut extra_bytes = vec![];
            //         for byte in reader.bytes() {
            //             match byte {
            //                 Err(_) => break,
            //                 Ok(b'\n') => {
            //                     extra_bytes.push(b'\n');
            //                     break;
            //                 }
            //                 Ok(b) => extra_bytes.push(b),
            //             }
            //         }
            //         // len += extra_bytes.len();
            //         chunk = [&buffer, &extra_bytes[..extra_bytes.len()]].concat();
            //     }

            //     // s1.send(chunk).unwrap();

            //     // Process the chunk
            //     // Note: You can use &chunk[..len] to get the slice containing valid data
            //     // Replace this comment with your processing logic
            // }
            // loop {
            //     // https://stackoverflow.com/questions/43028653/rust-file-i-o-is-very-slow-compared-with-c-is-something-wrong
            //     len = reader.read_until(b'\n', &mut buf).unwrap();

            //     if len == 0 {
            //         if lines.len() > 0 {
            //             s1.send(lines).unwrap();
            //         }
            //         break;
            //     }

            //     // Faster than collecting into buf and then splitting in the thread
            //     lines.push(buf[..len - n_eol_chars].to_vec());
            //     buf.clear();

            //     if lines.len() == max_lines {
            //         s1.send(lines).unwrap();
            //         lines = Vec::with_capacity(max_lines);
            //     }
            // }
            // force the receivers to close as well
            drop(s1);
            // let size = stdin.metadata().unwrap().len() as usize;

            // let mut reader = std::io::BufReader::with_capacity(1024 * 1024 * 1024, stdin);
            // let chunk_size = 200 * 1024 * 1024; // 200 MB
            // let mut chunk = Vec::with_capacity(chunk_size);

            // loop {
            //     let buffer = reader.fill_buf().unwrap();
            //     let buffer_len = buffer.len();

            //     if buffer_len == 0 {
            //         break; // End of file
            //     }

            //     let end_with_newline = buffer[buffer.len() - 1] == b'\n';

            //     // Copy the buffer to the chunk
            //     chunk.extend_from_slice(buffer);

            //     // Consume the buffer
            //     reader.consume(buffer_len);

            //     // If the buffer does not end with a newline, continue reading byte-by-byte
            //     if !end_with_newline {
            //         let mut temp_buf = [0u8; 1];
            //         loop {
            //             match reader.read(&mut temp_buf) {
            //                 Ok(0) | Err(_) => break, // End of stream or error
            //                 Ok(_) => {
            //                     let byte = temp_buf[0];
            //                     chunk.push(byte);

            //                     if byte == b'\n' {
            //                         break;
            //                     }
            //                 }
            //             }
            //         }
            //     }

            //     // If the chunk is full or ends with a newline, process it and clear it
            //     if end_with_newline {
            //         // Process the chunk here
            //         // ...
            //         // println!("CHUNK: {:?}", chunk.split(|byt| *byt == b'\n').count());
            //         s1.send(chunk.clone()).unwrap();

            //         // for line in chunk.split(|byt| *byt == b'\n') {
            //         //     // println!("LINE: {:?}", String::from_utf8(line.to_vec()))
            //         //     // lines.push(line.to_vec());
            //         //     // if lines.len() == max_lines {
            //         //     //     s1.send(lines).unwrap();
            //         //     //     lines = Vec::with_capacity(max_lines);
            //         //     // }
            //         // }
            //         // Clear the chunk for the next iteration
            //         chunk.clear();
            //     }
            // }

            // // Handle any remaining data in the chunk after the loop
            // if !chunk.is_empty() {
            //     // Process the remaining data in the chunk
            //     // ...
            //     println!("CHUNK: {:?}", chunk.split(|byt| *byt == b'\n').count());
            // }

            // // let mut reader = std::io::BufReader::with_capacity(32 * 1024 * 1024, stdin);
            // // let mut buf = Vec::with_capacity(size);
            // //     loop {
            // //         // https://stackoverflow.com/questions/43028653/rust-file-i-o-is-very-slow-compared-with-c-is-something-wrong
            // //         len = reader.read_until(b'\n', &mut buf).unwrap();

            // //         if len == 0 {
            // //             if lines.len() > 0 {
            // //                 s1.send(lines).unwrap();
            // //             }
            // //             break;
            // //         }

            // //         // Faster than collecting into buf and then splitting in the thread
            // //         lines.push(buf[..len - n_eol_chars].to_vec());
            // //         buf.clear();

            // //         if lines.len() == max_lines {
            // //             s1.send(lines).unwrap();
            // //             lines = Vec::with_capacity(max_lines);
            // //         }
            // //     }
            // //     // force the receivers to close as well
            // drop(s1);
        });

        for _ in 0..num_cpus::get() {
            let r = r1.clone();
            // necessary for borrow to work
            // let header = &header;
            scope.spawn(move |_| {
                process_lines(r, &head);
            });
        }
    })
    .unwrap();

    return Ok(());
}
