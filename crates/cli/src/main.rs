// #![allow(dead_code, unused_imports, unused_variables)]
use dungeon_core::exceptions::CustomErr;
use dungeon_core::{db::db::Db, io::file_source::FileSource, processor::process};

fn main() -> Result<(), CustomErr> {
    // Parse arguments
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => panic!("Missing <database path> and <command>"),
        2 => panic!("Missing <command>"),
        _ => {}
    }

    // Init pager and btree
    let src = FileSource::new(&args[1])?;
    let mut db = Db::open(src).expect("Error open db file");

    // Parse command and act accordingly
    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            // You can use print statements as follows for debugging, they'll be visible when running tests.
            eprintln!("Logs from your program will appear here!");

            // TODO: Uncomment the code below to pass the first stage
            println!("database page size: {}", db.page_size());
            println!("number of tables: {}", db.table_count());
        }
        ".tables" => {
            let mut table_names: Vec<&str> = Vec::new();
            for t in db.tables() {
                table_names.push(t.name());
            }
            table_names.sort();
            let names = table_names.join(" ");
            println!("{}", names);
        }
        // Support extra args for db exploration
        ".page" => {
            let page_no = &args[3].parse::<u64>().expect("Page no must be int");
            let page = db.page(*page_no).expect("Failed parsing page");
            println!("{}", page);
        }
        _ => {
            let output = process(&mut db, command.as_str());
            match output {
                Ok(v) => println!("{}", v),
                Err(e) => println!("ERR {:?}", e),
            }
        }
    }

    Ok(())
}
