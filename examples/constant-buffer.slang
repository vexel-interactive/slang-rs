struct Parameters
{
    float2 scale;
    float2 bias;
};

ConstantBuffer<Parameters> parameters;
RWStructuredBuffer<float2> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = parameters.scale * 2.0 + parameters.bias;
}
