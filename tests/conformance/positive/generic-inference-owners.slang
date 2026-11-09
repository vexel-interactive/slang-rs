struct Pair<T, U = int> { T first; U second; };
struct Outer<T>
{
    struct Inner<U> { T first; U second; };
    typealias Value = T;
};
struct A<T> { struct Inner { T value; }; };
struct B<T> { struct Inner { T value; }; };
int owner<T>(A<T>.Inner value) { return 1; }
int owner<T>(B<T>.Inner value) { return 2; }
T memberAlias<T>(Outer<T>.Value value) { return value; }
T first<T>(Pair<T> pair) { return pair.first; }
U second<T, U>(Outer<T>.Inner<U> value) { return value.second; }
T copy<T>(T value) { return (T)value; }
T at<T>(T value[2]) { return value[1]; }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Pair<int> pair = {2, 3};
    Outer<float>.Inner<int> inner = {4.0, 5};
    int values[2] = {6, 7};
    B<int>.Inner owned = {3};
    output[0] = uint(first(copy(pair)));
    output[1] = uint(second(inner));
    output[2] = uint(at(copy(values)));
    output[3] = uint(memberAlias(8));
    output[4] = uint(owner(owned));
}
