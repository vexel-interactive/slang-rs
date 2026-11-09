struct Outer
{
    [ForceInline] __init() { GroupMemoryBarrier(); }
    struct Inner { float value; };
};
[ForceInline] Outer make() { GroupMemoryBarrier(); return {}; }
[ForceInline] float read(Outer owner, Outer.Inner value = owner.Inner(2))
{
    return value.value;
}
[shader("fragment")]
float4 main() : SV_Target
{
    Outer.Inner first = make().Inner(3);
    Outer.Inner second = Outer().Inner(4);
    return float4(first.value + second.value);
}
