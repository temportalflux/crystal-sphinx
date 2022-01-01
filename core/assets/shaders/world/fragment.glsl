#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 frag_biome_color;
layout(location = 1) in vec2 frag_main_tex_coord;
layout(location = 2) in vec2 frag_biome_color_tex_coord;
layout(location = 3) in vec4 frag_flags;

// BlockType-based unform - bound based on which block type is being drawn
layout(set = 1, binding = 0) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

vec4 lerp_color(vec4 a, vec4 b, float t)
{
	return vec4(
		((1 - t) * a.rgb) + (t * b.rgb),
		((1 - t) * a.a) + (t * b.a)
	);
}

void main()
{
	if (frag_flags.r == 0) discard;
	
	// Sample the color for the main texture
	vec4 main_tex_color = texture(texSampler, frag_main_tex_coord);

	// Biome Colorization
	vec4 no_biome_color = vec4(1, 1, 1, 1);
	float biome_color_enabled = frag_flags.g;
	float biome_color_masked = frag_flags.b;
	float mask_for_biome_color = texture(texSampler, frag_biome_color_tex_coord).a;
	float mask_valid_for_frag = ((1 - biome_color_masked) * 1) + (biome_color_masked * mask_for_biome_color);
	float use_biome_color = biome_color_enabled * mask_valid_for_frag;
	vec4 biome_color = vec4(
		((1 - use_biome_color) * no_biome_color.rgb) + (use_biome_color * frag_biome_color.rgb),
		1.0
	);

	outColor = biome_color * main_tex_color;

	//outColor = vec4(use_biome_color, frag_flags.y, ceil(texture(texSampler, frag_biome_color_tex_coord).a), 1);
}