use engine::graphics::{
	chain::procedure::{AttachmentConfig, PhaseConfig, ProcedureConfig, ResourceConfig},
	flags::{
		Access, AttachmentKind, AttachmentOps, ImageLayout, ImageSampleKind, LoadOp, PipelineStage,
		SampleCount, StoreOp,
	},
	procedure::*,
	renderpass::ClearValue,
	resource::{depth_buffer::QueryResult, ColorBuffer, DepthBuffer, Registry},
	Chain,
};
use std::sync::{Arc, RwLock};

pub struct ChainConfig;
impl ProcedureConfig for ChainConfig {
	type Attachments = Attachments;
	type Phases = Phases;
	type Resources = Resources;
}

pub struct Attachments {
	frame: Arc<Attachment>,
	color_buffer: Arc<Attachment>,
	depth_buffer: Arc<Attachment>,
	depth_query: QueryResult,
}

impl AttachmentConfig for Attachments {
	fn new(chain: &Chain) -> anyhow::Result<Self> {
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

		Ok(Self {
			frame,
			color_buffer,
			depth_buffer,
			depth_query,
		})
	}

	fn swapchain_attachment(&self) -> &Arc<Attachment> {
		&self.frame
	}
}

pub struct Phases {
	pub world: Arc<Phase>,
	pub debug: Arc<Phase>,
	pub resolve_antialiasing: Arc<Phase>,
	pub ui: Arc<Phase>,
	pub egui: Arc<Phase>,
}
impl PhaseConfig<Attachments> for Phases {
	fn new(attachments: &Attachments) -> anyhow::Result<Self> {
		let world = Arc::new(
			Phase::new("World")
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
					attachment::Reference::from(&attachments.color_buffer)
						.with_kind(AttachmentKind::Color)
						.with_layout(ImageLayout::ColorAttachmentOptimal),
				)
				.with_attachment(
					attachment::Reference::from(&attachments.depth_buffer)
						.with_kind(AttachmentKind::DepthStencil)
						.with_layout(ImageLayout::DepthStencilAttachmentOptimal),
				),
		);

		let debug = Arc::new(
			Phase::new("Debug")
				.with_dependency(
					Dependency::new(Some(&world))
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
					attachment::Reference::from(&attachments.color_buffer)
						.with_kind(AttachmentKind::Color)
						.with_layout(ImageLayout::ColorAttachmentOptimal),
				)
				.with_attachment(
					attachment::Reference::from(&attachments.depth_buffer)
						.with_kind(AttachmentKind::DepthStencil)
						.with_layout(ImageLayout::DepthStencilAttachmentOptimal),
				),
		);

		let resolve_antialiasing = Arc::new(
			Phase::new("Resolve Antialiasing")
				.with_dependency(
					Dependency::new(Some(&debug))
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
					attachment::Reference::from(&attachments.color_buffer)
						.with_kind(AttachmentKind::Color)
						.with_layout(ImageLayout::ColorAttachmentOptimal),
				)
				.with_attachment(
					attachment::Reference::from(&attachments.frame)
						.with_kind(AttachmentKind::Resolve)
						.with_layout(ImageLayout::ColorAttachmentOptimal),
				),
		);

		let ui = Arc::new(
			Phase::new("UI")
				.with_dependency(
					Dependency::new(Some(&resolve_antialiasing))
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
					attachment::Reference::from(&attachments.frame)
						.with_kind(AttachmentKind::Color)
						.with_layout(ImageLayout::ColorAttachmentOptimal),
				),
		);

		let egui = Arc::new(
			Phase::new("EGui")
				.with_dependency(
					Dependency::new(Some(&ui))
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
					attachment::Reference::from(&attachments.frame)
						.with_kind(AttachmentKind::Color)
						.with_layout(ImageLayout::ColorAttachmentOptimal),
				),
		);

		Ok(Self {
			world,
			debug,
			resolve_antialiasing,
			ui,
			egui,
		})
	}

	fn apply_to(&self, procedure: &mut Procedure) -> anyhow::Result<()> {
		procedure.add_phase(self.world.clone())?;
		procedure.add_phase(self.debug.clone())?;
		procedure.add_phase(self.resolve_antialiasing.clone())?;
		procedure.add_phase(self.ui.clone())?;
		procedure.add_phase(self.egui.clone())?;
		Ok(())
	}
}

pub struct Resources;
impl ResourceConfig<Attachments> for Resources {
	fn create_resources(attachments: Attachments, resources: &mut Registry) -> anyhow::Result<()> {
		resources.add(
			ColorBuffer::builder()
				.with_attachment(attachments.color_buffer)
				.build(),
		);
		resources.add(
			DepthBuffer::builder()
				.with_query(attachments.depth_query)
				.with_attachment(attachments.depth_buffer)
				.build(),
		);
		Ok(())
	}
}
