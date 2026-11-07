struct FragmentInput
{
    float4 position : SV_Position;
    nointerpolation uint material : MATERIAL;
};

[shader("pixel")]
float4 main(FragmentInput input, bool front : SV_IsFrontFace) : SV_Target
{
    return float4(float(input.material), float(front), 0.0, 1.0);
}
