use luamoon::{
	ast::{lexer::Lexer, parser::{TokenIterator, parse}},
	compiler::compile,
	lua_lib::{pcall, print},
	vm::{execute, value::{Table, Value}}
};
use maplit::hashmap;
use std::{env::args, fs::File, io::Read};

fn main() {
	let args = args().collect::<Vec<_>>();

	let mut code = String::new();
	File::open(&args[2]).unwrap().read_to_string(&mut code).unwrap();

	match &args[1] as &str {
		"lex" => {
			let tokens = Lexer {source: code.chars().peekable()};

			println!("{:#?}", tokens.collect::<Vec<_>>());
		},
		"parse" => {
			let tokens = Lexer {source: code.chars().peekable()};
			let block = parse(&mut TokenIterator(tokens.peekable()));

			match block {
				Ok(block) => println!("{:#?}", block),
				Err(error) => println!("SYNTAX ERROR: {}", error)
			}
		},
		"compile" => {
			let tokens = Lexer {source: code.chars().peekable()};
			let block = parse(&mut TokenIterator(tokens.peekable()));
			let function = match block {
				Ok(block) => compile(&block),
				Err(error) => return println!("SYNTAX ERROR: {}", error)
			};

			println!("{:#?}", function)
		},
		"run" => {
			let tokens = Lexer {source: code.chars().peekable()};
			let block = parse(&mut TokenIterator(tokens.peekable()));
			let function = match block {
				Ok(block) => compile(&block),
				Err(error) => return println!("SYNTAX ERROR: {}", error)
			};

			let locals = hashmap! {
				Value::String("print".to_owned().into_boxed_str()) =>
					Value::NativeFunction(print),
			};

			let globals = Table::from_hashmap(hashmap! {
				Value::new_string("print") =>
					Value::NativeFunction(print),
				Value::new_string("pcall") =>
					Value::NativeFunction(pcall)
			}).arc();

			{
				let mut data = globals.data.lock().unwrap();
				data.insert(Value::new_string("_G"), Value::Table(globals.clone()));
			}

			match execute(&function.into(), locals, globals) {
				Ok(_) => (),
				Err(error) => println!("ERROR: {}", error)
			}
		},
		_ => println!("unknown verb")
	}
}
