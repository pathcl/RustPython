// use std::ops::Deref;
use std::collections::HashMap;
use std::io::{self, Write};

use super::compile;
use super::pyobject::DictProtocol;
use super::pyobject::{PyContext, PyObject, PyObjectKind, PyObjectRef, PyResult, Scope, IdProtocol, PyFuncArgs};
use super::vm::VirtualMachine;
use super::objbool;


fn get_locals(vm: &mut VirtualMachine) -> PyObjectRef {
    let mut d = vm.new_dict();
    // TODO: implement dict_iter_items?
    let locals = vm.get_locals();
    match locals.borrow().kind {
        PyObjectKind::Dict { ref elements } => {
            for l in elements {
                d.set_item(l.0, l.1.clone());
            }
        }
        _ => {}
    };
    d
}

fn dir_locals(vm: &mut VirtualMachine) -> PyObjectRef {
    get_locals(vm)
}

fn dir_object(vm: &mut VirtualMachine, obj: PyObjectRef) -> PyObjectRef {
    let d = vm.new_dict();
    d
}

pub fn builtin_dir(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    if args.args.is_empty() {
        Ok(dir_locals(vm))
    } else {
        let obj = args.args.into_iter().next().unwrap();
        Ok(dir_object(vm, obj))
    }
}

pub fn builtin_id(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    if args.args.len() != 1 {
        return Err(vm.new_exception("Expected only one argument".to_string()))
    }

    Ok(vm.context().new_int(args.args[0].get_id() as i32))
}

pub fn builtin_print(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    trace!("print called with {:?}", args);
    for a in args.args {
        print!("{} ", a.borrow().str());
    }
    println!();
    io::stdout().flush().unwrap();
    Ok(vm.get_none())
}

pub fn builtin_compile(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    if args.args.len() < 1 {
        return Err(vm.new_exception("Expected more arguments".to_string()))
    }
    // TODO:
    let mode = compile::Mode::Eval;
    let source = args.args[0].borrow().str();

    match compile::compile(vm, &source, mode) {
        Ok(value) => Ok(value),
        Err(msg) => Err(vm.new_exception(msg)),
    }
}

pub fn builtin_eval(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    let args = args.args;
    if args.len() > 3 {
        return Err(vm.new_exception("Expected at maximum of 3 arguments".to_string()))
    } else if args.len() > 2 {
        // TODO: handle optional global and locals
    } else {
        return Err(vm.new_exception("Expected at least one argument".to_string()))
    }
    let source = args[0].clone();
    let _globals = args[1].clone();
    let locals = args[2].clone();

    let code_obj = source; // if source.borrow().kind 

    // Construct new scope:
    let scope_inner = Scope {
        locals: locals,
        parent: None,
    };
    let scope = PyObject { kind: PyObjectKind::Scope { scope: scope_inner }, typ: None }.into_ref();

    // Run the source:
    vm.run_code_obj(code_obj, scope)
}

pub fn builtin_locals(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    Ok(vm.get_locals())
}

pub fn builtin_len(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    if args.args.len() != 1 {
        panic!("len(s) expects exactly one parameter");
    }
    let len = match args.args[0].borrow().kind {
        PyObjectKind::List { ref elements } => elements.len(),
        PyObjectKind::Tuple { ref elements } => elements.len(),
        PyObjectKind::String { ref value } => value.len(),
        _ => {
            return Err(vm.context()
                .new_str("TypeError: object of this type has no len()".to_string()))
        }
    };
    Ok(vm.context().new_int(len as i32))
}

pub fn make_module(ctx: &PyContext) -> PyObjectRef {
    // scope[String::from("print")] = print;
    let mut dict = HashMap::new();
    dict.insert(String::from("id"), ctx.new_rustfunc(builtin_id));
    dict.insert(String::from("print"), ctx.new_rustfunc(builtin_print));
    dict.insert(String::from("type"), ctx.type_type.clone());
    dict.insert(String::from("int"), ctx.int_type.clone());
    dict.insert(String::from("all"), ctx.new_rustfunc(builtin_all));
    dict.insert(String::from("any"), ctx.new_rustfunc(builtin_any));
    dict.insert(String::from("dir"), ctx.new_rustfunc(builtin_dir));
    dict.insert(String::from("locals"), ctx.new_rustfunc(builtin_locals));
    dict.insert(String::from("compile"), ctx.new_rustfunc(builtin_compile));
    dict.insert(String::from("eval"), ctx.new_rustfunc(builtin_eval));
    dict.insert("len".to_string(), ctx.new_rustfunc(builtin_len));
    let d2 = PyObject::new(PyObjectKind::Dict { elements: dict }, ctx.type_type.clone());
    let scope = PyObject::new(PyObjectKind::Scope { scope: Scope { locals: d2, parent: None} }, ctx.type_type.clone());
    let obj = PyObject::new(
        PyObjectKind::Module {
            name: "__builtins__".to_string(),
            dict: scope,
        },
        ctx.type_type.clone(),
    );
    obj
}

fn builtin_any(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    Ok(vm.new_bool(args.args.into_iter().any(|e| objbool::boolval(e))))
}

fn builtin_all(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    Ok(vm.new_bool(args.args.into_iter().all(|e| objbool::boolval(e))))
}
