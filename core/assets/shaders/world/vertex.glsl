#version 450
#extension GL_ARB_separate_shader_objects : enable

// The number of blocks in each dimension of a chunk.
// MIRRORS: `chunk::SIZE`
vec3 CHUNK_SIZE = vec3(16.0, 16.0, 16.0);

// Camera-based unform - changes each frame based on the camera's POV and chunk data
layout(set = 0, binding = 0) uniform CameraUniform {
	mat4 view;
	mat4 proj;
	vec3 posOfCurrentChunk;
} camera;

// Model attributes - changes based on the block type being drawn
layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tex_coord;
layout(location = 2) in vec4 model_flags;

// Instance attributes - changes based on a specific block being drawn
layout(location = 3) in vec3 chunk_coordinate;
layout(location = 4) in mat4 model_matrix; // slots [4,8)
layout(location = 8) in vec4 instance_flags;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 fragTexCoord;
layout(location = 2) out vec4 fragFlags;

highp int bitSubset(int field, int size, int start, int end)
{
	int shifted = int(field);
	shifted <<= (size - end - 1);
	shifted >>= (size - end - 1);
	shifted >>= start;
	return shifted;
}

void main()
{
	vec3 chunk_offset = chunk_coordinate - camera.posOfCurrentChunk;
	// component-wise multiply to convert chunk pos to world pos
	vec3 blockPosRelativeToCameraChunk = chunk_offset * CHUNK_SIZE;
	vec3 vertPos = blockPosRelativeToCameraChunk + position;
	gl_Position = camera.proj * camera.view * model_matrix * vec4(vertPos, 1.0);
	
	// bit mask indicating what face this is
	int faceMask = floatBitsToInt(model_flags.x) & 0x3F; // 0b111111
	int faceEnabledBits = floatBitsToInt(instance_flags.x);
	fragFlags.x = float(faceMask & faceEnabledBits);
	
	fragColor = vec4(1, 1, 1, 1);
	fragTexCoord = tex_coord;
}