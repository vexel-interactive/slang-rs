struct Parameters
{
    float first;
    row_major float2x3 skipped[0];
    float second;
    float nested[3][0];
    Texture2D<float4> textures[0];
    float third;
};
ConstantBuffer<Parameters> input;
float prefix;
float empty[0];
float scale;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = (input.first + input.second + input.third) * scale + prefix;
}
