use engine::channels::mpsc::{Receiver, Sender};
use engine::math::nalgebra::Point3;

use crate::block;

pub type OperationSender = Sender<Operation>;
pub type OperationReceiver = Receiver<Operation>;
pub enum Operation {
	Remove(Point3<i64>),
	Insert(Point3<i64>, Vec<(Point3<usize>, block::LookupId)>),
}
