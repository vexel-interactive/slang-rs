T copy<T = int>(T value) { return value; }
T relay<T>(T value) { return copy(value); }
T pick<T>(T first, T second) { return second; }
U partial<T, U = T>(T first, U second = U()) { return second; }
void assign<T>(out T result, T value) { result = value; }
void exchange<T>(inout T first, inout T second)
{
    T old = first;
    first = second;
    second = old;
}
vector<T, 1 + 1> shaped<T>(vector<T, 2> value) { return value; }
vector<T, 2> shapedPick<T>(vector<T, 2> first, vector<T, 2> second) { return second; }
typealias Element<T> = T;
Element<T> aliased<T>(Element<T> value) { return value; }
T choose<T>(T value) { return value; }
int choose(int value) { return value + 10; }

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    int first = 2;
    int second = 3;
    output[0] = uint(copy<>(first));
    output[1] = uint(copy(3.0));
    output[2] = uint(pick(2, 4.0));
    output[3] = uint(pick(2, int2(3, 5)).y);
    output[4] = uint(partial<int>(2, 6.0));
    output[5] = uint(partial(2.0));
    output[6] = uint(relay(copy(shaped(int2(7, 8)))).y);
    assign(first, 9);
    exchange(first, second);
    output[7] = uint(first);
    output[8] = uint(second);
    output[9] = uint(choose(2));
    output[10] = uint(choose(4.0));
    output[11] = uint(pick<int2>({2, 3}, int2(4, 5)).y);
    output[12] = uint(aliased(13));
    output[13] = uint(shapedPick({2, 3}, int2(4, 5)).y);
}
