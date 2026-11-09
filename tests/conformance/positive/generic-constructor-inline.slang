static int count = 0;
struct Marker
{
    int value;
    [ForceInline] __init<T>(T ignored, int input)
    {
        for (int i = 0; i < 2; i++) count++;
        value = input;
        if (input > 0) return;
        value = 13;
    }
};
static Marker global = Marker(int2(1, 2), 5);
Marker create<T>(T input) { return Marker(input, 7); }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(global.value);
    output[1] = uint(create(float2(1, 2)).value);
    output[2] = uint(Marker(int2(1, 2), 0).value);
    output[3] = uint(count);
}
