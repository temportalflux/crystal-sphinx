#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 frag_tex_coord;

layout(set = 1, binding = 0) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

void main()
{
	// Sample the color for the main texture
	outColor = texture(texSampler, frag_tex_coord);
}