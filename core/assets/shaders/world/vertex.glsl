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
	
	// Determine if the face should be drawn
	// -------------------------------------
	// Bit-Mask which indicates which of the 6 faces this vertex is on
	int faceMask = floatBitsToInt(model_flags.x) & 0x3F; // 0b111111
	// Get the bit-mask for which faces are enabled/visible for this instance
	int faceEnabledBits = floatBitsToInt(instance_flags.x);
	// Tell the fragment shader if this fragment is actually visible;
	// aka is the face this vertex is on enabled for the instance.
	// 0.0 means the face/vertex is not visible and should be discarded.
	// 1.0 means the face IS visible and should be draw.
	fragFlags.x = float(ceil(faceMask & faceEnabledBits));
	
	// NOTE: Will eventually be used to colorize voxel faces based on biome
	fragColor = vec4(1, 1, 1, 1);

	// Copy over the texture coordinate for sampling from atlas
	fragTexCoord = tex_coord;
}