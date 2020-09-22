// sliceobject.{h,c} in CPython

use super::objint::PyInt;
use super::objtype::PyClassRef;
use crate::function::{OptionalArg, PyFuncArgs};
use crate::pyobject::{
    BorrowValue, IntoPyObject, PyClassImpl, PyComparisonValue, PyContext, PyObjectRef, PyRef,
    PyResult, PyValue, TryIntoRef, TypeProtocol,
};
use crate::slots::{Comparable, Hashable, PyComparisonOp, Unhashable};
use crate::VirtualMachine;
use num_bigint::{BigInt, ToBigInt};
use num_traits::{One, Signed, Zero};

#[pyclass(module = false, name = "slice")]
#[derive(Debug)]
pub struct PySlice {
    pub start: Option<PyObjectRef>,
    pub stop: PyObjectRef,
    pub step: Option<PyObjectRef>,
}

impl PyValue for PySlice {
    fn class(vm: &VirtualMachine) -> PyClassRef {
        vm.ctx.types.slice_type.clone()
    }
}

pub type PySliceRef = PyRef<PySlice>;

#[pyimpl(with(Hashable, Comparable))]
impl PySlice {
    #[pyproperty(name = "start")]
    fn start(&self, vm: &VirtualMachine) -> PyObjectRef {
        self.start.clone().into_pyobject(vm)
    }

    #[pyproperty(name = "stop")]
    fn stop(&self, _vm: &VirtualMachine) -> PyObjectRef {
        self.stop.clone()
    }

    #[pyproperty(name = "step")]
    fn step(&self, vm: &VirtualMachine) -> PyObjectRef {
        self.step.clone().into_pyobject(vm)
    }

    #[pymethod(name = "__repr__")]
    fn repr(&self, vm: &VirtualMachine) -> PyResult<String> {
        let start = self.start(vm);
        let stop = self.stop(vm);
        let step = self.step(vm);

        let start_repr = vm.to_repr(&start)?;
        let stop_repr = vm.to_repr(&stop)?;
        let step_repr = vm.to_repr(&step)?;

        Ok(format!(
            "slice({}, {}, {})",
            start_repr.borrow_value(),
            stop_repr.borrow_value(),
            step_repr.borrow_value()
        ))
    }

    pub fn start_index(&self, vm: &VirtualMachine) -> PyResult<Option<BigInt>> {
        if let Some(obj) = &self.start {
            to_index_value(vm, obj)
        } else {
            Ok(None)
        }
    }

    pub fn stop_index(&self, vm: &VirtualMachine) -> PyResult<Option<BigInt>> {
        to_index_value(vm, &self.stop)
    }

    pub fn step_index(&self, vm: &VirtualMachine) -> PyResult<Option<BigInt>> {
        if let Some(obj) = &self.step {
            to_index_value(vm, obj)
        } else {
            Ok(None)
        }
    }

    #[pyslot]
    fn tp_new(cls: PyClassRef, args: PyFuncArgs, vm: &VirtualMachine) -> PyResult<PySliceRef> {
        let slice: PySlice = match args.args.len() {
            0 => {
                return Err(
                    vm.new_type_error("slice() must have at least one arguments.".to_owned())
                );
            }
            1 => {
                let stop = args.bind(vm)?;
                PySlice {
                    start: None,
                    stop,
                    step: None,
                }
            }
            _ => {
                let (start, stop, step): (PyObjectRef, PyObjectRef, OptionalArg<PyObjectRef>) =
                    args.bind(vm)?;
                PySlice {
                    start: Some(start),
                    stop,
                    step: step.into_option(),
                }
            }
        };
        slice.into_ref_with_type(vm, cls)
    }

    pub(crate) fn inner_indices(
        &self,
        length: &BigInt,
        vm: &VirtualMachine,
    ) -> PyResult<(BigInt, BigInt, BigInt)> {
        // Calculate step
        let step: BigInt;
        if vm.is_none(&self.step(vm)) {
            step = One::one();
        } else {
            // Clone the value, not the reference.
            let this_step: PyRef<PyInt> = self.step(vm).try_into_ref(vm)?;
            step = this_step.borrow_value().clone();

            if step.is_zero() {
                return Err(vm.new_value_error("slice step cannot be zero.".to_owned()));
            }
        }

        // For convenience
        let backwards = step.is_negative();

        // Each end of the array
        let lower = if backwards {
            -1_i8.to_bigint().unwrap()
        } else {
            Zero::zero()
        };

        let upper = if backwards {
            lower.clone() + length
        } else {
            length.clone()
        };

        // Calculate start
        let mut start: BigInt;
        if vm.is_none(&self.start(vm)) {
            // Default
            start = if backwards {
                upper.clone()
            } else {
                lower.clone()
            };
        } else {
            let this_start: PyRef<PyInt> = self.start(vm).try_into_ref(vm)?;
            start = this_start.borrow_value().clone();

            if start < Zero::zero() {
                // From end of array
                start += length;

                if start < lower {
                    start = lower.clone();
                }
            } else if start > upper {
                start = upper.clone();
            }
        }

        // Calculate Stop
        let mut stop: BigInt;
        if vm.is_none(&self.stop(vm)) {
            stop = if backwards { lower } else { upper };
        } else {
            let this_stop: PyRef<PyInt> = self.stop(vm).try_into_ref(vm)?;
            stop = this_stop.borrow_value().clone();

            if stop < Zero::zero() {
                // From end of array
                stop += length;
                if stop < lower {
                    stop = lower;
                }
            } else if stop > upper {
                stop = upper;
            }
        }

        Ok((start, stop, step))
    }

    #[pymethod(name = "indices")]
    fn indices(&self, length: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        if let Some(length) = length.payload::<PyInt>() {
            let (start, stop, step) = self.inner_indices(length.borrow_value(), vm)?;
            Ok(vm.ctx.new_tuple(vec![
                vm.ctx.new_int(start),
                vm.ctx.new_int(stop),
                vm.ctx.new_int(step),
            ]))
        } else {
            Ok(vm.ctx.not_implemented())
        }
    }
}

impl Comparable for PySlice {
    fn cmp(
        zelf: PyRef<Self>,
        other: PyObjectRef,
        op: PyComparisonOp,
        vm: &VirtualMachine,
    ) -> PyResult<PyComparisonValue> {
        let other = class_or_notimplemented!(Self, other);

        let ret = match op {
            PyComparisonOp::Lt | PyComparisonOp::Le => None
                .or_else(|| vm.bool_seq_lt(zelf.start(vm), other.start(vm)).transpose())
                .or_else(|| vm.bool_seq_lt(zelf.stop(vm), other.stop(vm)).transpose())
                .or_else(|| vm.bool_seq_lt(zelf.step(vm), other.step(vm)).transpose())
                .unwrap_or_else(|| Ok(op == PyComparisonOp::Le))?,
            PyComparisonOp::Eq | PyComparisonOp::Ne => {
                let eq = vm.identical_or_equal(&zelf.start(vm), &other.start(vm))?
                    && vm.identical_or_equal(&zelf.stop(vm), &other.stop(vm))?
                    && vm.identical_or_equal(&zelf.step(vm), &other.step(vm))?;
                if op == PyComparisonOp::Ne {
                    !eq
                } else {
                    eq
                }
            }
            PyComparisonOp::Gt | PyComparisonOp::Ge => None
                .or_else(|| vm.bool_seq_gt(zelf.start(vm), other.start(vm)).transpose())
                .or_else(|| vm.bool_seq_gt(zelf.stop(vm), other.stop(vm)).transpose())
                .or_else(|| vm.bool_seq_gt(zelf.step(vm), other.step(vm)).transpose())
                .unwrap_or_else(|| Ok(op == PyComparisonOp::Ge))?,
        };

        Ok(PyComparisonValue::Implemented(ret))
    }
}

impl Unhashable for PySlice {}

fn to_index_value(vm: &VirtualMachine, obj: &PyObjectRef) -> PyResult<Option<BigInt>> {
    if vm.is_none(obj) {
        return Ok(None);
    }

    let result = vm.to_index(obj).unwrap_or_else(|| {
        Err(vm.new_type_error(
            "slice indices must be integers or None or have an __index__ method".to_owned(),
        ))
    })?;
    Ok(Some(result.borrow_value().clone()))
}

#[pyclass(module = false, name = "EllipsisType")]
#[derive(Debug)]
pub struct PyEllipsis;

impl PyValue for PyEllipsis {
    fn class(vm: &VirtualMachine) -> PyClassRef {
        vm.ctx.ellipsis.class()
    }
}

#[pyimpl]
impl PyEllipsis {
    #[pyslot]
    fn tp_new(_cls: PyClassRef, vm: &VirtualMachine) -> PyRef<Self> {
        vm.ctx.ellipsis.clone()
    }

    #[pymethod(magic)]
    fn repr(&self) -> String {
        "Ellipsis".to_owned()
    }

    #[pymethod(magic)]
    fn reduce(&self) -> String {
        "Ellipsis".to_owned()
    }
}

pub fn init(context: &PyContext) {
    PySlice::extend_class(context, &context.types.slice_type);
    PyEllipsis::extend_class(context, &context.ellipsis.class());
}
