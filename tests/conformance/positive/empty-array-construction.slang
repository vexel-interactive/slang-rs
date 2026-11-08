typedef float EmptyFloats[0];
typedef int EmptyInts[0];
float choose(EmptyFloats value) { return 1; }
float choose(EmptyInts value) { return 2; }
struct Parameters { EmptyFloats values; float scale = 3; };
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    EmptyFloats a = float[0]();
    EmptyFloats b = a;
    EmptyInts c = int[0]();
    float nested[2][0] = float[0][2]();
    Parameters p = Parameters(b);
    result[0] = choose(b) + choose(c) + p.scale;
}
