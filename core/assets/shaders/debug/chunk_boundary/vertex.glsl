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
layout(location = 1) in vec4 color;
layout(location = 2) in vec4 flags;

// Instance attributes
layout(location = 3) in mat4 model_matrix; // slots [3,7)

layout(location = 0) out vec4 fragColor;

float lerp(float a, float b, float delta) { return (a * (1 - delta)) + (b * delta); }
vec3 lerp(vec3 a, vec3 b, vec3 delta) { return vec3(lerp(a.x, b.x, delta.x), lerp(a.y, b.y, delta.y), lerp(a.z, b.z, delta.z)); }
vec4 lerp(vec4 a, vec4 b, float delta) { return (a * (1 - delta)) + (b * delta); }

void main()
{
	// This is the vertex position shifted out of the camera's chunk and into world space
	//vec3 pos_wrt_rootChunk = position + ((vec3(0, 0, 0) - camera.posOfCurrentChunk) * CHUNK_SIZE);
	vec4 pos_v4 = vec4(position, 1.0);
	vec4 pos_in_camera_space = lerp(
		// Exists in world space, apply transforms: model->world, then world->camera
		camera.view * model_matrix * pos_v4,
		// Exists in camera space, apply transforms: inverse camera rotation, then model->camera
		model_matrix * (camera.inv_rotation * pos_v4),
		flags.x
	);
	gl_Position = camera.proj * pos_in_camera_space;
	fragColor = color;
}