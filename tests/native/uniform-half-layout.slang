struct Parameters
{
    float prefix;
    half scalar;
    half2 pair;
    bool enabled;
    float3x1 column;
    float after;
};

ConstantBuffer<Parameters> parameters;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = parameters.prefix + float(parameters.scalar) + float(parameters.pair.x)
        + float(parameters.enabled) + parameters.column[0][0] + parameters.after;
}
