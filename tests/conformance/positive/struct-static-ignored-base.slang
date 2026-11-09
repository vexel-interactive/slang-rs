struct S
{
    static const float value = 2.0;
    [ForceInline] __init() { GroupMemoryBarrier(); }
};
[ForceInline] S make() { GroupMemoryBarrier(); return {}; }
[shader("fragment")]
float4 main() : SV_Target
{
    return float4(S().value + make().value);
}
