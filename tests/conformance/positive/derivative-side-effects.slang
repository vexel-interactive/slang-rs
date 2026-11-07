static uint evaluations;

float read(float value)
{
    evaluations++;
    return value;
}

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target
{
    float width = fwidth_coarse(read(uv.x)) + fwidth_fine(read(uv.y));
    return float4(width + float(evaluations));
}
