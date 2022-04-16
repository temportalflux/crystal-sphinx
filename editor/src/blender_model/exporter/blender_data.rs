use crate::blender_model::{
	exporter::{Model, Vertex, VertexWeight},
	Point, Polygon,
};
use anyhow::Context;
use engine::math::nalgebra::{Vector2, Vector3};
use tokio::{io::AsyncReadExt, process::ChildStdout};

pub struct BlenderData {
	stream: ChildStdout,
	points: Vec<Point>,
	polygons: Vec<Polygon>,
}

impl BlenderData {
	pub fn new(stream: ChildStdout) -> Self {
		Self {
			stream,
			points: Vec::new(),
			polygons: Vec::new(),
		}
	}

	pub async fn process(mut self) -> anyhow::Result<Model> {
		let vertex_count = self.stream.read_u32().await? as usize;
		self.points = Vec::with_capacity(vertex_count);
		for idx in 0..vertex_count {
			let vertex = Point::read(&mut self.stream).await.context(format!(
				"failed to read point idx={idx} of {vertex_count} points"
			))?;
			self.points.push(vertex);
		}

		let polygon_count = self.stream.read_u32().await? as usize;
		self.polygons = Vec::with_capacity(polygon_count);
		for _ in 0..polygon_count {
			let polygon = Polygon::read(&mut self.stream)
				.await
				.context("failed to read polygon")?;
			self.polygons.push(polygon);
		}

		self.into_model()
	}

	fn into_model(self) -> anyhow::Result<Model> {
		let mut vertices = VertexSet::with_capacity(self.points.len());
		let mut indices = Vec::new();
		for polygon in self.polygons.iter() {
			for (vertex_index, tex_coord) in polygon.vertices.iter() {
				indices.push(vertices.get_or_insert((*vertex_index, polygon.normal, *tex_coord)));
			}
		}
		let (vertices, vertex_weights) = vertices
			.into_inner()
			.into_iter()
			.map(|(vertex_index, normal, tex_coord)| {
				let point = &self.points[vertex_index];
				let vertex = Vertex {
					position: point.position,
					normal,
					tex_coord,
				};
				let groups = point
					.groups
					.iter()
					.map(|group| VertexWeight {
						group_index: group.group_index,
						weight: group.weight,
					})
					.collect::<Vec<_>>();
				(vertex, groups)
			})
			.unzip();

		Ok(Model {
			vertices,
			indices,
			vertex_weights,
		})
	}
}

struct VertexSet(Vec<(usize, Vector3<f32>, Vector2<f32>)>);
impl VertexSet {
	fn with_capacity(size: usize) -> Self {
		Self(Vec::with_capacity(size))
	}

	fn get_or_insert(&mut self, vertex: (usize, Vector3<f32>, Vector2<f32>)) -> usize {
		match self.0.iter().rev().position(|vert| *vert == vertex) {
			Some(idx) => idx,
			None => {
				let idx = self.0.len();
				self.0.push(vertex);
				idx
			}
		}
	}

	fn into_inner(self) -> Vec<(usize, Vector3<f32>, Vector2<f32>)> {
		self.0
	}
}
