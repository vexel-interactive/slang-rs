struct Marker
{
    int value;
    __init<T>(T ignored) { value = 17; }
};
struct Mixed
{
    int value;
    __init(int input) { value = input + 1; }
    __init<T>(T ignored) { value = 25; }
};
struct Defaulted
{
    int value;
    __init<T=int, U=T>(T ignored, U input=U()) { value = 31; }
};
struct Prototype
{
    int value;
    __init<T>(T input);
    __init<U>(U renamed) { value = 37; }
};
struct Output
{
    int value;
    __init<T>(T ignored, out int previous, int input)
    {
        previous = input;
        value = input + 2;
    }
};
static int calls = 0;
int index() { calls++; return 0; }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(Marker(3).value);
    output[1] = uint(Marker(3.0).value);
    output[2] = uint(Marker(int2(3, 4)).value);
    output[3] = uint(Marker().value);
    Marker source = Marker(2);
    source.value = 9;
    output[4] = uint(Marker(source).value);
    Marker fromList = {source};
    output[5] = uint(fromList.value);
    Marker numericList = {4};
    output[6] = uint(numericList.value);
    output[7] = uint(Mixed(4).value);
    output[8] = uint(Mixed(4.0).value);
    output[9] = uint(Defaulted(float2(2, 3)).value);
    output[10] = uint(Prototype(int2(2, 3)).value);
    int values[2] = {0, 0};
    Output changed = Output(int2(1, 2), values[index()], 41);
    output[11] = uint(changed.value + values[0] + calls);
}
