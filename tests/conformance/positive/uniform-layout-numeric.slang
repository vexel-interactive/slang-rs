struct Numeric
{
    float scalar;
    float2 pair;
    float3 triple;
    float tail;
    double2 widePair;
    double3 wideTriple;
    float afterWide;
    float2x3 matrix;
    float afterMatrix;
    bool enabled;
    int count;
    uint mask;
};

ConstantBuffer<Numeric> parameters;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = parameters.scalar + parameters.pair.x + parameters.triple.x
        + parameters.tail + float(parameters.widePair.x) + float(parameters.wideTriple.x)
        + parameters.afterWide + parameters.matrix[0][1] + parameters.afterMatrix
        + float(parameters.enabled) + float(parameters.count) + float(parameters.mask);
}
