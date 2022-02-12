use engine::math::nalgebra::Point3;

use crate::block;

pub type OperationSender = crossbeam_channel::Sender<Operation>;
pub type OperationReceiver = crossbeam_channel::Receiver<Operation>;
pub enum Operation {
	Remove(Point3<i64>),
	Insert(Point3<i64>, Vec<(Point3<usize>, block::LookupId)>),
}
