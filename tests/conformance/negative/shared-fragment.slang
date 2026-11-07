groupshared uint values[2];

[shader("fragment")]
float4 main(nointerpolation uint index : TEXCOORD0) : SV_Target
{
    values[index & 1u] = index;
    uint old;
    InterlockedAdd(values[(index + 1u) & 1u], 1u, old);
    return float4(old);
}
