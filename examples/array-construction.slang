typedef float Row[3];
struct Sample { float value = 7.0; };
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Row row = Row(1.0, 2.0);
    Row copy = float[3](row);
    Sample defaults[2] = Sample[2]();
    float nested[2][3] = float[3][2](Row(1.0, 2.0, 3.0), {4.0, 5.0, 6.0});
    float flat[2][3] = float[3][2](1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    result[0] = copy[2] + defaults[1].value + nested[1][2] + flat[1][2];
}
