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

// Model attributes - changes based on the block type being drawn
layout(location = 0) in vec3 position;
layout(location = 1) in vec4 tex_coord;
layout(location = 2) in vec4 model_flags;

// Instance attributes - changes based on a specific block being drawn
layout(location = 3) in vec3 chunk_coordinate;
layout(location = 4) in mat4 model_matrix; // slots [4,8)
layout(location = 8) in vec4 instance_flags;

layout(location = 0) out vec4 frag_biome_color;
layout(location = 1) out vec2 frag_main_tex_coord;
layout(location = 2) out vec2 frag_biome_color_tex_coord;
layout(location = 3) out vec4 frag_flags;

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

	int model_flags1 = floatBitsToInt(model_flags.x);
	int instance_flags1 = floatBitsToInt(instance_flags.x);
	
	// Determine if the face should be drawn
	// -------------------------------------
	// Bit-Mask which indicates which of the 6 faces this vertex is on
	int faceMask = model_flags1 & 0x3F; // 0b111111
	// Get the bit-mask for which faces are enabled/visible for this instance
	int faceEnabledBits = instance_flags1 & 0x3F;
	// Tell the fragment shader if this fragment is actually visible;
	// aka is the face this vertex is on enabled for the instance.
	// 0.0 means the face/vertex is not visible and should be discarded.
	// 1.0 means the face IS visible and should be draw.
	frag_flags.r = float(ceil(faceMask & faceEnabledBits));

	// Extract the flag indicating if the vertex supports colorizing
	int biome_color_enabled = model_flags1 & (1 << 6); // the bit directly after the face-mask bits
	int biome_color_masked = model_flags1 & (1 << 7);
	// 0.0 if colorizing is disabled
	// 1.0 if colorizing is enabled
	float colorizing_enabled = float(min(biome_color_enabled, 1));
	frag_flags.g = float(min(biome_color_enabled, 1));
	frag_flags.b = float(min(biome_color_masked, 1));
	
	// NOTE: Will eventually be used to colorize voxel faces based on biome
	vec3 biome_color = vec3(85.0 / 255.0, 201.0 / 255.5, 63.0 / 255.0); // 0x55C93F
	frag_biome_color = vec4(biome_color, 1.0);

	// Copy over the texture coordinate for sampling from atlas
	frag_main_tex_coord = tex_coord.rg;
	frag_biome_color_tex_coord = tex_coord.ba;
}