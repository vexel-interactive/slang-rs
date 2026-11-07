struct Params
{
    float scale;
};

float value;
ConstantBuffer<Params> first;
ConstantBuffer<Params> second;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = value + first.scale + second.scale;
}
