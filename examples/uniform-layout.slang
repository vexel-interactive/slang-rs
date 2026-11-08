struct Pair
{
    float x;
    float y;
};

struct Parameters
{
    float z;
    Pair pair;
    int b;
    float2 values[2];
    int tail;
};

ConstantBuffer<Parameters> parameters;
uniform float prefix;
uniform Parameters data;
uniform float after;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = prefix + data.z + data.pair.x + data.values[1].x
        + float(data.tail) + after + parameters.pair.y;
}
