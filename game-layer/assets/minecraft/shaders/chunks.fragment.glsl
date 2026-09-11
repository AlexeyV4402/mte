#version 460

layout(location = 0) in vec2 in_uv;
layout(location = 1) flat in uint in_material_id;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform texture2DArray t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;

void main() {
    vec3 uvw = vec3(in_uv, float(in_material_id));
    
    out_color = texture(sampler2DArray(t_diffuse, s_diffuse), uvw);
}