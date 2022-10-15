#version 450
#extension GL_ARB_separate_shader_objects : enable

// The number of blocks in each dimension of a chunk.
// MIRRORS: `chunk::SIZE`
vec3 CHUNK_SIZE = vec3(16.0, 16.0, 16.0);

// Camera-based unform - changes each frame based on the camera's POV and chunk data
layout(set = 0, binding = 0) uniform CameraUniform {
	mat4 view;
	mat4 proj;
	mat4 inv_rotation;
	vec3 posOfCurrentChunk;
} camera;

// Model attributes - changes based on the entity model being drawn
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 tex_coord;

// Instance attributes - changes based on the specific entity being drawn
layout(location = 3) in vec3 chunk_coordinate;
layout(location = 4) in mat4 model_matrix; // slots [4,8)

layout(location = 0) out vec2 frag_tex_coord;

void main()
{
	// How far away from the camera's current chunk is the chunk of the voxel being rendered?
	vec3 chunk_offset = chunk_coordinate - camera.posOfCurrentChunk;
	// Convert the chunk distance into a number of blocks
	vec3 blockPosRelativeToCameraChunk = chunk_offset * CHUNK_SIZE;
	// Now add the position of the block inside the chunk to the number of blocks from the camera's chunk
	vec3 vertPos = blockPosRelativeToCameraChunk + position;
	// Integrate the vertex model matrix with its block-offset position
	// and the camera's view (which includes the camera's offset in its chunk) and projection.
	// This results in the virtual position of the block, on the screen,
	// relative to the camera's view (position & orientation).
	gl_Position = camera.proj * camera.view * model_matrix * vec4(vertPos, 1.0);

	frag_tex_coord = tex_coord.rg;
}