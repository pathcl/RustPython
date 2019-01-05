/*
 * I/O core tools.
 */


use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader,BufWriter};

use super::super::obj::objstr;
use super::super::obj::objint;
use num_bigint::{BigInt, ToBigInt};

use super::super::obj::objtype;

use super::super::pyobject::{
    PyContext, PyFuncArgs, PyObjectKind, PyObjectRef, PyResult, TypeProtocol, AttributeProtocol
};

use super::super::vm::VirtualMachine;

use num_traits::ToPrimitive;

fn string_io_init(vm: &mut VirtualMachine, _args: PyFuncArgs) -> PyResult {
    // arg_check!(vm, args, required = [(s, Some(vm.ctx.str_type()))]);
    // TODO
    Ok(vm.get_none())
}

fn string_io_getvalue(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(vm, args);
    // TODO
    Ok(vm.get_none())
}

fn bytes_io_init(vm: &mut VirtualMachine, _args: PyFuncArgs) -> PyResult {
    // TODO
    Ok(vm.get_none())
}

fn bytes_io_getvalue(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(vm, args);
    // TODO
    Ok(vm.get_none())
}

fn buffered_io_base_init(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    // arg_check!(vm, args);

    // TODO
    Ok(vm.get_none())
}

fn file_io_init(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(
        vm,
        args,
        required = [(file_io, None), (name, Some(vm.ctx.str_type()))]
    );

    vm.ctx.set_attr(&file_io, "name", name.clone());
    vm.ctx.set_attr(&file_io, "__offset__", vm.ctx.new_int(0.to_bigint().unwrap()));
    Ok(vm.get_none())
}

fn file_io_read(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(
        vm,
        args,
        required = [(file_io, None)]
    );
    let py_name = file_io.get_attr("name").unwrap();
    let f = match File::open(objstr::get_value(& py_name)) {
        Ok(v) => Ok(v),
        Err(v) => Err(vm.new_type_error("Error opening file".to_string())),
    }; 

    let buffer = match f {
        Ok(v) =>  Ok(BufReader::new(v)),
        Err(v) => Err(vm.new_type_error("Error reading from file".to_string()))
    };

    let mut bytes = vec![];
    if let Ok(mut buff) = buffer {
        buff.read_to_end(&mut bytes);
    }    

    Ok(vm.ctx.new_bytes(bytes))
}


fn file_io_readinto(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(
        vm,
        args,
        required = [(file_io, None), (obj, Some(vm.ctx.bytearray_type()))]
    );

    //extract length of buffer
    let len_method = vm.get_method(obj.clone(), &"__len__".to_string());
    let py_length = vm.invoke(len_method.unwrap(), PyFuncArgs::default());
    let length = objint::get_value(&py_length.unwrap()).to_u64().unwrap();

    let py_offset = vm.ctx.get_attr(&file_io, &"__offset__".to_string());
    let offset = objint::get_value(&py_offset.unwrap()).to_u64().unwrap();

    //extract file name and open a file handle, taking length many bytes
    let py_name = file_io.get_attr("name").unwrap();

    let handle = match File::open(objstr::get_value(&py_name)) {
        Ok(v) => Ok(v.take(offset + length)),
        Err(v) => Err(vm.new_type_error("Error opening file".to_string())),
    }; 
    
    if let Ok(mut f) = handle {
        match obj.borrow_mut().kind {
            PyObjectKind::Bytes { ref mut value } => {
                value.clear();
                f.read_to_end(&mut *value);
                *value = value.iter().skip(offset as usize).cloned().collect();
            },
            _ => {}
        };
    }

    vm.ctx.set_attr(&file_io, "__offset__", vm.ctx.new_int(offset+length.to_bigint().unwrap()));

    Ok(vm.get_none())
}

fn buffered_reader_read(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(vm, args);

    //simple calls read on the read class!
    // TODO
    Ok(vm.get_none())
}

fn io_open(vm: &mut VirtualMachine, args: PyFuncArgs) -> PyResult {
    arg_check!(
        vm, 
        args, 
        required = [(file, None), (mode, None)]
    );


    Ok(vm.get_none())

    //mode is optional: 'rt' is the default mode (open from reading text)
    //To start we construct a FileIO (subclass of RawIOBase)
    //This is subsequently consumed by a Buffered_class of type depending
    //operation in the mode. i.e:
    // updating => PyBufferedRandom
    // creating || writing || appending => BufferedWriter
    // reading => BufferedReader
    // If the mode is binary this Buffered class is returned directly at
    // this point.
    //For Buffered class construct "raw" IO class e.g. FileIO and pass this into corresponding field

    //If the mode is text this buffer type is consumed on construction of 
    //a TextIOWrapper which is subsequently returned.
}


pub fn mk_module(ctx: &PyContext) -> PyObjectRef {
    let py_mod = ctx.new_module(&"io".to_string(), ctx.new_scope(None));
    ctx.set_attr(&py_mod, "open", ctx.new_rustfunc(io_open));
     //IOBase the abstract base class of the IO Module
    let io_base = ctx.new_class("IOBase", ctx.object());
    ctx.set_attr(&py_mod, "IOBase", io_base.clone());

    // IOBase Subclasses
    let raw_io_base = ctx.new_class("RawIOBase", ctx.object());
    ctx.set_attr(&py_mod, "RawIOBase", raw_io_base.clone());

    let buffered_io_base = ctx.new_class("BufferedIOBase", io_base.clone());
    ctx.set_attr(&buffered_io_base, "__init__", ctx.new_rustfunc(buffered_io_base_init));
    ctx.set_attr(&py_mod, "BufferedIOBase", buffered_io_base.clone());

    let text_io_base = ctx.new_class("TextIOBase", io_base.clone());
    ctx.set_attr(&py_mod, "TextIOBase", text_io_base.clone());

    // RawBaseIO Subclasses
    let file_io = ctx.new_class("FileIO", raw_io_base.clone());
    ctx.set_attr(&file_io, "__init__", ctx.new_rustfunc(file_io_init));
    ctx.set_attr(&file_io, "name", ctx.str_type());
    ctx.set_attr(&file_io, "read", ctx.new_rustfunc(file_io_read));
    ctx.set_attr(&file_io, "readinto", ctx.new_rustfunc(file_io_readinto));
    ctx.set_attr(&py_mod, "FileIO", file_io.clone());

    // BufferedIOBase Subclasses
    let buffered_reader = ctx.new_class("BufferedReader", buffered_io_base.clone());
    ctx.set_attr(&py_mod, "BufferedReader", buffered_reader.clone());

    let buffered_reader = ctx.new_class("BufferedWriter", buffered_io_base.clone());
    ctx.set_attr(&py_mod, "BufferedWriter", buffered_reader.clone());

    //TextIOBase Subclass
    let text_io_wrapper = ctx.new_class("TextIOWrapper", ctx.object());
    ctx.set_attr(&py_mod, "TextIOWrapper", text_io_wrapper.clone());

    // BytesIO: in-memory bytes
    let string_io = ctx.new_class("StringIO", io_base.clone());
    ctx.set_attr(&string_io, "__init__", ctx.new_rustfunc(string_io_init));
    ctx.set_attr(&string_io, "getvalue", ctx.new_rustfunc(string_io_getvalue));
    ctx.set_attr(&py_mod, "StringIO", string_io);

    // StringIO: in-memory text
    let bytes_io = ctx.new_class("BytesIO", io_base.clone());
    ctx.set_attr(&bytes_io, "__init__", ctx.new_rustfunc(bytes_io_init));
    ctx.set_attr(&bytes_io, "getvalue", ctx.new_rustfunc(bytes_io_getvalue));
    ctx.set_attr(&py_mod, "BytesIO", bytes_io);

    py_mod
}