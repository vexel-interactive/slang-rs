struct Parameters
{
    float2x3 matrix : packoffset(c2);
    float after;
};
ConstantBuffer<Parameters> params : register(b3);
float evaluate(ConstantBuffer<Parameters> input)
{
    return input.matrix[0][1] + input.after;
}
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = evaluate(params); }
