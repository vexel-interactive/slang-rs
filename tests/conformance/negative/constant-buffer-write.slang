struct Parameters
{
    float value;
};

ConstantBuffer<Parameters> parameters;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    parameters.value = 2.0;
}
