struct Parameters { float value; };
ConstantBuffer<Parameters> input;
RWStructuredBuffer<float> output;
float read(Parameters value) { return value.value; }
float choose(Parameters value) { return value.value; }
uint choose(ConstantBuffer<Parameters> value) { return 1u; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    Parameters value = input;
    output[0] = read(input) + value.value + choose(Parameters(input));
}
