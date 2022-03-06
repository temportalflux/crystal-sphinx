use engine::graphics::{
	flags::{
		Access, AttachmentKind, AttachmentOps, ImageLayout, ImageSampleKind, LoadOp, PipelineStage,
		SampleCount, StoreOp,
	},
	procedure::*,
	renderpass::ClearValue,
	resource::{ColorBuffer, DepthBuffer},
	Chain,
};
use std::sync::Arc;

pub mod chunk_boundary;
pub mod model;
pub mod voxel;

pub fn initialize_chain(chain: &mut Chain) -> anyhow::Result<()> {
	let viewport_format = chain.swapchain_image_format();
	let max_common_samples = chain
		.physical()?
		.max_common_sample_count(ImageSampleKind::Color | ImageSampleKind::Depth)
		.unwrap_or(SampleCount::_1);

	let frame = Arc::new(
		Attachment::default()
			.with_format(viewport_format)
			.with_general_ops(AttachmentOps {
				load: LoadOp::DontCare,
				store: StoreOp::Store,
			})
			.with_final_layout(ImageLayout::PresentSrc)
			.with_clear_value(ClearValue::Color([0.0, 0.0, 0.0, 1.0])),
	);

	let color_buffer = Arc::new(
		Attachment::default()
			.with_format(viewport_format)
			.with_sample_count(max_common_samples)
			.with_general_ops(AttachmentOps {
				load: LoadOp::Clear,
				store: StoreOp::Store,
			})
			.with_final_layout(ImageLayout::ColorAttachmentOptimal)
			.with_clear_value(ClearValue::Color([0.0, 0.0, 0.0, 1.0])),
	);

	let depth_query = DepthBuffer::classic_format_query().query(&chain.physical()?)?;
	let depth_buffer = Arc::new(
		Attachment::default()
			.with_format(depth_query.format())
			.with_sample_count(max_common_samples)
			.with_general_ops(AttachmentOps {
				load: LoadOp::Clear,
				store: StoreOp::DontCare,
			})
			.with_stencil_ops(AttachmentOps {
				load: LoadOp::DontCare,
				store: StoreOp::DontCare,
			})
			.with_final_layout(ImageLayout::DepthStencilAttachmentOptimal)
			.with_clear_value(ClearValue::DepthStencil(1.0, 0)),
	);

	chain.set_procedure(create_procedure(&frame, &color_buffer, &depth_buffer)?);

	chain.resources_mut().add(
		ColorBuffer::builder()
			.with_attachment(color_buffer.clone())
			.build(),
	);
	chain.resources_mut().add(
		DepthBuffer::builder()
			.with_query(depth_query)
			.with_attachment(depth_buffer.clone())
			.build(),
	);

	Ok(())
}

fn create_procedure(
	frame: &Arc<Attachment>,
	color_buffer: &Arc<Attachment>,
	depth_buffer: &Arc<Attachment>,
) -> anyhow::Result<engine::graphics::procedure::Procedure> {
	let world_phase = Arc::new(
		Phase::new()
			.with_dependency(
				Dependency::new(None)
					.first(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_stage(PipelineStage::EarlyFragmentTests),
					)
					.then(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_stage(PipelineStage::EarlyFragmentTests)
							.with_access(Access::ColorAttachmentWrite)
							.with_access(Access::DepthStencilAttachmentWrite),
					),
			)
			.with_attachment(
				attachment::Reference::from(color_buffer)
					.with_kind(AttachmentKind::Color)
					.with_layout(ImageLayout::ColorAttachmentOptimal),
			)
			.with_attachment(
				attachment::Reference::from(depth_buffer)
					.with_kind(AttachmentKind::DepthStencil)
					.with_layout(ImageLayout::DepthStencilAttachmentOptimal),
			),
	);

	let debug_phase = Arc::new(
		Phase::new()
			.with_dependency(
				Dependency::new(Some(&world_phase))
					.first(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_stage(PipelineStage::EarlyFragmentTests)
							.with_access(Access::ColorAttachmentWrite),
					)
					.then(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_stage(PipelineStage::EarlyFragmentTests)
							.with_access(Access::ColorAttachmentWrite)
							.with_access(Access::DepthStencilAttachmentRead),
					),
			)
			.with_attachment(
				attachment::Reference::from(color_buffer)
					.with_kind(AttachmentKind::Color)
					.with_layout(ImageLayout::ColorAttachmentOptimal),
			)
			.with_attachment(
				attachment::Reference::from(depth_buffer)
					.with_kind(AttachmentKind::DepthStencil)
					.with_layout(ImageLayout::DepthStencilAttachmentOptimal),
			),
	);

	let resolve_antialiasing_phase = Arc::new(
		Phase::new()
			.with_dependency(
				Dependency::new(Some(&debug_phase))
					.first(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_access(Access::ColorAttachmentWrite),
					)
					.then(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_access(Access::ColorAttachmentWrite),
					),
			)
			.with_attachment(
				attachment::Reference::from(color_buffer)
					.with_kind(AttachmentKind::Color)
					.with_layout(ImageLayout::ColorAttachmentOptimal),
			)
			.with_attachment(
				attachment::Reference::from(frame)
					.with_kind(AttachmentKind::Resolve)
					.with_layout(ImageLayout::ColorAttachmentOptimal),
			),
	);

	let ui_phase = Arc::new(
		Phase::new()
			.with_dependency(
				Dependency::new(Some(&resolve_antialiasing_phase))
					.first(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_access(Access::ColorAttachmentWrite),
					)
					.then(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_access(Access::ColorAttachmentWrite),
					),
			)
			.with_attachment(
				attachment::Reference::from(frame)
					.with_kind(AttachmentKind::Color)
					.with_layout(ImageLayout::ColorAttachmentOptimal),
			),
	);

	let egui_phase = Arc::new(
		Phase::new()
			.with_dependency(
				Dependency::new(Some(&ui_phase))
					.first(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_access(Access::ColorAttachmentWrite),
					)
					.then(
						PhaseAccess::default()
							.with_stage(PipelineStage::ColorAttachmentOutput)
							.with_access(Access::ColorAttachmentWrite),
					),
			)
			.with_attachment(
				attachment::Reference::from(frame)
					.with_kind(AttachmentKind::Color)
					.with_layout(ImageLayout::ColorAttachmentOptimal),
			),
	);

	Ok(Procedure::default()
		.with_phase(world_phase)?
		.with_phase(debug_phase)?
		.with_phase(resolve_antialiasing_phase)?
		.with_phase(ui_phase)?
		.with_phase(egui_phase)?)
}
