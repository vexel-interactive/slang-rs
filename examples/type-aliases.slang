typedef float Number;
typealias Vector = float2;
typedef Number Row[2];
typealias Grid = Row[3];

RWStructuredBuffer<Number> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Grid values;
    values[2][1] = 3.0;
    Vector pair = Vector(Number(2), values[2][1]);
    result[0] = pair.x + pair.y;
}
