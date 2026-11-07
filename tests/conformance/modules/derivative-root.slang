import "derivative-library.slang";

float fwidth_coarse(float value) { return value; }

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target
{
    return float4(measure(uv.x) + fwidth_coarse(uv.y));
}
