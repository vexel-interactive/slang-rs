uniform float _slang_globals;
uniform float globals;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = _slang_globals + globals;
}
