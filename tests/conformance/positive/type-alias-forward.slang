typealias Number = Later;
typedef float32_t Later;
typealias Vector = float2;

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Vector value = Vector(Number(1), Number(2));
    result[0] = value.x + value.y;
}
