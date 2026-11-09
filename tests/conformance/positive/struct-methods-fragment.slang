struct Ignored
{
    int value;
    __init(int x) { GroupMemoryBarrier(); value = x; }
    static float read(float x) { return x + 1; }
};
struct Derivative
{
    float value;
    [ForceInline] float slope() { return ddx(value); }
};
[shader("fragment")]
float4 main(float x : TEXCOORD0) : SV_Target
{
    return float4(Ignored(2).read(x) + Derivative(x).slope());
}
