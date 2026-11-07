struct FragmentInput
{
    float4 position : SV_Position;
    float2 uv : TEXCOORD0;
};

[shader("fragment")]
float4 main(FragmentInput input) : SV_Target0
{
    return float4(input.uv, 0.0, 1.0);
}
