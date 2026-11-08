struct Parameters
{
    float scalar;
    float2 values[2][3];
    float tail;
};

typealias Data = Parameters;
ConstantBuffer<Data> parameters[2];
uniform Data data[2];
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = parameters[0].scalar + parameters[1].tail
        + data[0].values[1][2].x + data[1].tail;
}
