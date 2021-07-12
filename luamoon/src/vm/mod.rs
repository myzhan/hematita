pub mod constant;
pub mod value;

use self::{
	constant::Constant,
	value::{Function, IntoNillableValue, Nil, NonNil, Table, Value}
};
use if_chain::if_chain;
use std::{collections::HashMap, sync::Arc};

/// Executes a function.
pub fn execute(function: &Function, mut local: HashMap<Value, Value>,
		global: Arc<Table>) -> Result<Arc<Table>, String> {
	let mut index = 0; // The current opcode we're evaluating.
	// current_opcode

	fn retrieve(key: &Value, local: &mut HashMap<Value, Value>,
			global: &Arc<Table>) -> Option<Value> {
		match local.get(key) {
			Some(value) => Some(value.clone()),
			None => {
				let global = global.data.lock().unwrap();
				global.get(key).cloned()
			}
		}
	}

	loop {
		if index == function.chunk.opcodes.len() {break Ok(Table::default().arc())}

		match function.chunk.opcodes[index] {
			OpCode::Call {arguments, function, destination, ..} => {
				let args = match retrieve(&Value::new_string(arguments),
						&mut local, &global).nillable() {
					NonNil(Value::Table(table)) => table,
					// args is not a table.
					args => break Err(format!(
						"attempt to initiate a function call with a {} value",
							args.type_name()))
				};

				match retrieve(&Value::new_string(function),
						&mut local, &global).nillable() {
					NonNil(Value::NativeFunction(func)) => {
						let result = func(args.clone(), global.clone())?;
						let result = result.data.lock().unwrap();
						
						let destination = Value::new_string(destination);
						match result.get(&Value::Integer(1)) {
							Some(result) => {local.insert(destination, result.clone());},
							None => {local.remove(&destination);}
						}
					},
					NonNil(Value::Function(func)) => {
						// TODO: Make locals and globals tables...
						let result = execute(&func, args.data.lock().unwrap().clone(), global.clone())?;
						let result = result.data.lock().unwrap();
						
						let destination = Value::new_string(destination);
						match result.get(&Value::Integer(1)) {
							Some(result) => {local.insert(destination, result.clone());},
							None => {local.remove(&destination);}
						}
					},
					// func is not a function.
					func => break Err(format!("attempt to call a {} value", func.type_name()))
				}
			},

			OpCode::IndexRead {indexee, index, destination, ..} =>
					match retrieve(&Value::new_string(indexee), &mut local, &global)
						.nillable() {
				// indexee is a table.
				NonNil(Value::Table(table)) => {
					let index = retrieve(&Value::new_string(index), &mut local, &global)
						.ok_or("table index is nil".to_owned())?; // table index is nil.
					let table = table.data.lock().unwrap();
					let value = table.get(&index).map(Clone::clone);
					drop(table); // Borrow checker stuff.

					let destination = Value::new_string(destination);
					// value is not nil.
					if let Some(value) = value {
						local.insert(destination, value);
					// value is nil.
					} else {
						local.remove(&destination);
					}
				},
				// indexee is not a table.
				indexee => break Err(format!(
					"attempt to index a {} value", indexee.type_name()))
			},

			OpCode::IndexWrite {indexee, index, value} =>
					match retrieve(&Value::new_string(indexee), &mut local, &global)
						.nillable() {
				// indexee is a table.
				NonNil(Value::Table(table)) => {
					let mut lock = table.data.lock().unwrap();
					let index = retrieve(&Value::new_string(index), &mut local, &global)
						.ok_or("table index is nil".to_owned())?; // index is nil.

					// value is not nil.
					if let Some(value) = retrieve(&Value::new_string(value),
							&mut local, &global) {
						lock.insert(index.clone(), value.clone());
					// value is nil.
					} else {
						lock.remove(&index);
					}
				},
				// indexee is not a table.
				indexee => break Err(format!(
					"attempt to index a {} value", indexee.type_name()))
			},

			OpCode::Load {constant, destination, ..} =>
					match function.chunk.constants.get(constant as usize) {
				// constant is not nil.
				Some(constant) => {local.insert(Value::new_string(destination),
					constant.clone().into_value());},
				// constant is.... nil?
				None => {local.remove(&Value::new_string(destination));}
			},

			OpCode::ReAssign {actor, destination, ..} => {
				let actor = retrieve(&Value::new_string(actor), &mut local, &global);
				match retrieve(&actor.unwrap(), &mut local, &global).nillable() {
					NonNil(value) => {
						let value = value.clone();
						local.insert(Value::new_string(destination), value);
					},
					Nil => {local.remove(&Value::new_string(destination));}
				}
			},

			OpCode::Create {destination, ..} =>
				{local.insert(Value::new_string(destination),
					Value::Table(Arc::default()));},

			OpCode::BinaryOperation {first, second, destination, operation, ..} => {
				let first = retrieve(&Value::new_string(first),
					&mut local, &global).nillable();
				let second = retrieve(&Value::new_string(second),
					&mut local, &global).nillable();
				let destination = Value::new_string(destination);

				// Uneeded when 53667 and 51114.
				// In order to have the match statement continue after we actually get
				// the function object, we currently have to use a block within the if
				// statements condition, primarily because 51114 isn't implemented yet,
				// which would allow us to check the contents of the Mutex during match
				// and save a variable with it's contents. 53667 needs to be implemented
				// to, because we also need to pattern match through an Arc.
				let mut transfer_result = None;
				let result = match operation {
					// BinaryOperation::Equal

					// BinaryOPeration::NotEqual

					// BinaryOperation::LessThan

					BinaryOperation::LessThanOrEqual => match (&first, &second) {
						(NonNil(Value::Integer(first)), NonNil(Value::Integer(second))) =>
							Value::Boolean(first <= second),
						(NonNil(Value::String(first)), NonNil(Value::String(second))) =>
							Value::Boolean(first <= second),
						/*
							// Code for when features are added...
							if let Some(metamethod) = &metamethod.metatable &&
								if let NonNil(metamethod) = metamethod.data.lock().unwrap()
									.get(&Value::identifier("__le")).nillable() => {
						*/
						(NonNil(Value::Table(metamethod)), _) if {
							if_chain! {
								if let Some(metamethod) = &metamethod.metatable;
								if let NonNil(metamethod) = metamethod.data.lock().unwrap()
									.get(&Value::new_string("__le")).nillable();
								then {
									match metamethod {
										Value::Function(metamethod) => {
											let mut arguments = HashMap::new();
											if let NonNil(first) = first.cloned()
												{arguments.insert(Value::Integer(0), first);}
											if let NonNil(second) = second.cloned()
												{arguments.insert(Value::Integer(0), second);}
											transfer_result = Some(execute(
												&*metamethod, arguments, global.clone())?);
											true
										},
										Value::NativeFunction(metamethod) => {
											transfer_result = Some(metamethod(Table::array(
												[&first, &second]).arc(), global.clone())?);
											true
										},
										_ => false
									}
								} else {false}
							}
						} => {
							// Panic is impossible because we put in something in the if.
							transfer_result.unwrap().data.lock().unwrap()
								.get(&Value::Integer(0)).nillable().coerce_to_boolean()
						},
						/*
							// Code for when features are added...
							if let Some(metamethod) = &metamethod.metatable &&
								if let NonNil(metamethod) = metamethod.data.lock().unwrap()
									.get(&Value::identifier("__le")).nillable() => {
						*/
						(_, NonNil(Value::Table(metamethod))) if {
							if_chain! {
								if let Some(metamethod) = &metamethod.metatable;
								if let NonNil(metamethod) = metamethod.data.lock().unwrap()
									.get(&Value::new_string("__le")).nillable();
								then {
									match metamethod {
										Value::Function(metamethod) => {
											let mut arguments = HashMap::new();
											if let NonNil(first) = first.cloned()
												{arguments.insert(Value::Integer(0), first);}
											if let NonNil(second) = second.cloned()
												{arguments.insert(Value::Integer(0), second);}
											transfer_result = Some(execute(
												&*metamethod, arguments, global.clone())?);
											true
										},
										Value::NativeFunction(metamethod) => {
											transfer_result = Some(metamethod(Table::array(
												[&first, &second]).arc(), global.clone())?);
											true
										},
										_ => false
									}
								} else {false}
							}
						} => {
							// Panic is impossible because we put in something in the if.
							transfer_result.unwrap().data.lock().unwrap()
								.get(&Value::Integer(0)).nillable().coerce_to_boolean()
						},
						_ => todo!()
					},
					
					// BinaryOperation::GreaterThan

					// BinaryOperation::LessThan

					// BinaryOperation::Add

					// BinaryOperation::Subtract

					_ => todo!()
				};

				local.insert(destination, result);
			},

			OpCode::UnaryOperation {operand, operation, destination, ..} => {
				let operand = retrieve(&Value::new_string(operand),
					&mut local, &global).nillable();
				let destination = Value::new_string(destination);

				let result = match operation {
					UnaryOperation::Not => Value::Boolean(!operand.coerce_to_bool())
				};

				local.insert(destination, result);
			},

			OpCode::Jump {operation, r#if: None} => {
				index = operation as usize;
				continue;
			},

			OpCode::Jump {operation, r#if: Some(check)} => {
				let check = retrieve(&Value::new_string(check), &mut local, &global);
				if check.nillable().coerce_to_bool() {
					index = operation as usize;
					continue
				}
			},

			OpCode::Return {result} => match retrieve(
					&Value::new_string(result), &mut local, &global).nillable() {
				NonNil(Value::Table(result)) => break Ok(result.clone()),
				_ => panic!()
			},

			OpCode::NoOp => ()
		}

		index = index + 1;
	}
}

/// The operation codes used within the lua virtual machine. All lua code is
/// compiled to blocks of opcodes. Opcodes are the primitive block of "action"
/// (lua code ran from this vm cannot perform any actions more specific than
/// what these opcodes can provide). Each opcode does a different thing, but
/// most opcodes operate with memory directly from the local scope via symbols.
///
/// Typically, the lua compiler will use temporary variable names that cannot
/// be accessed from lua directly. All of these temporary variables start with
/// a left parenthesis, because that makes these variables impossible to access
/// or alter accidentally in lua. Theoretically, you can use methods from the
/// debug module to access these temporary variables, but tampering with them
/// won't do much more than corrupt the state of the currently executing
/// function.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum OpCode<'s> {
	/// Calls a function with the name [function], with the arguments array
	/// [arguments], and stores the result array in [destination]. [arguments]
	/// must be a lua array (a table, typically with numbered keys starting from
	/// 1), or else an error will be thrown.
	///
	/// [destination_local] determines if the result will be stored to the local
	/// scope or global scope.
	Call {
		/// The name of the function, in scope, to be called. Must be a function
		/// or else an error will be thrown.
		function: &'s str,
		/// The name of the arguments array, in scope. Must be a table or else an
		/// error will be thrown.
		arguments: &'s str,
		/// The name of where the return values will be stored.
		destination: &'s str,
		/// Whether or not the return values will be stored in the local scope or
		/// global scope.
		destination_local: bool
	},

	/// Indexes into the object with the name [indexee], with index [index], and
	/// stores the result in [destination].
	///
	/// [destination_local] determines if the result will be stored to the local
	/// scope or global scope.
	IndexRead {
		/// The name of the object to be indexed.
		indexee: &'s str,
		/// The name of the object that serves as the index.
		index: &'s str,
		/// The name of where the result will be stored.
		destination: &'s str,
		/// Whether or not the result will be stored in the local scope or global
		/// scope.
		destination_local: bool
	},

	/// Indexes into the object with the name [indexee], with index [index], and
	/// writes [value] into [indexee].
	IndexWrite {
		/// The name of the object to be indexed.
		indexee: &'s str,
		/// The name of the object that serves as the index.
		index: &'s str,
		/// The name of the value to be stored within the indexee.
		value: &'s str
	},

	/// Loads a value from the constant pool at index [constant] to [destination].
	///
	/// [destination_local] determines if the constant will be stored to the local
	/// scope or global scope.
	Load {
		/// The constant to be loaded.
		constant: u16,
		/// The name of where the constant will be stored.
		destination: &'s str,
		/// Whether or not the constant will be stored in the local scope or global
		/// scope.
		destination_local: bool
	},

	ReAssign {
		actor: &'s str,
		destination: &'s str,
		destination_local: bool
	},

	/// Creates a new empty table at [destination].
	///
	/// [destination_local] determines if the new table will be stored to the
	/// local scope or global scope.
	Create {
		/// The name of where the new table will be stored.
		destination: &'s str,
		/// Whether or not the new table will be stored in the local scope or global
		/// scope.
		destination_local: bool
	},

	BinaryOperation {
		first: &'s str,
		second: &'s str,
		operation: BinaryOperation,
		destination: &'s str,
		local: bool
	},

	UnaryOperation {
		operand: &'s str,
		operation: UnaryOperation,
		destination: &'s str,
		local: bool
	},

	/// Jumps unconditionally to [operation], or conditionally if the name of a
	/// condition is specified in [r#if]. The jump operation is performed in
	/// number of opcodes, not bytes.
	Jump {
		/// The opcode to jump to.
		operation: u64,
		/// An optional condition. If specified, the value at the specified name
		/// must be true or false, otherwise an error will be thrown. If true, the
		/// jump will occur, otherwise it will not.
		r#if: Option<&'s str>
	},

	Return {
		result: &'s str
	},
	
	NoOp
}

// TODO: Should we remove [crate::ast::parser::BinaryOperator] and use this
// instead? Same goes for UnaryOperation and Operator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryOperation {
	Equal,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	Add,
	Subtract
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnaryOperation {
	Not
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Chunk {
	pub constants: Vec<Constant>,
	pub opcodes: Vec<OpCode<'static>>
}

impl Chunk {
	pub fn arc(self) -> Arc<Self> {
		Arc::new(self)
	}
}
