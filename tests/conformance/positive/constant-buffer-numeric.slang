ConstantBuffer<float> scalar;
ConstantBuffer<float3> vector;
ConstantBuffer<row_major float2x3> matrix;
RWStructuredBuffer<float> output;
float read(ConstantBuffer<float> value) { return value + 1; }
float read(float value) { return value + 2; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    float x = scalar;
    float3 y = vector;
    float2x3 z = matrix;
    output[0] = read(scalar) + sin(scalar) + vector.y + matrix[1][2]
        + x + y.z + z[0][1];
}
