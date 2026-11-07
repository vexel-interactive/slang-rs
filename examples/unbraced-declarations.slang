RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    if (id.x == 0)
        float value = 2.0, next = value + 1.0;
    else
        value = next = 4.0;

    int seed = 0;
    do
        int iteration = seed++;
    while (iteration < 2);

    while (seed < 5)
        float step = float(++seed);
    step = float(seed);

    for (int i = 0; i < 2; i++)
        float temporary = step + float(i);

    result[id.x] = value + next + step + float(iteration);
}
