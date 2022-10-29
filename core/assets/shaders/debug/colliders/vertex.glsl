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

// Model attributes
layout(location = 0) in vec3 position;
layout(location = 1) in vec4 vertex_color;
layout(location = 2) in vec4 flags;

// Instance attributes
layout(location = 3) in vec3 chunk_coordinate;
layout(location = 4) in mat4 model_matrix; // slots [3,7)
layout(location = 8) in vec4 instance_color;

layout(location = 0) out vec4 fragColor;

void main()
{
	vec4 pos_v4 = model_matrix * vec4(position, 1.0);

	// How far away from the camera's current chunk is the chunk of the voxel being rendered?
	vec3 chunk_offset = chunk_coordinate - camera.posOfCurrentChunk;
	// Convert the chunk distance into a number of blocks
	vec3 blockPosRelativeToCameraChunk = chunk_offset * CHUNK_SIZE;
	// Now add the position of the block inside the chunk to the number of blocks from the camera's chunk
	vec3 vertPos = blockPosRelativeToCameraChunk + pos_v4.xyz;

	gl_Position = camera.proj * camera.view * vec4(vertPos, 1.0);
	fragColor = vertex_color * instance_color;
}