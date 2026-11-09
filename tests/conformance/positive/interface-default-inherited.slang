interface Base
{
    This combine(This other);
    [mutating] void update(This other) { this = combine(other); }
}
interface Left : Base {}
interface Right : Base {}
interface Derived : Left, Right {}
struct Value : Derived
{
    int value;
    __init(int input) { value = input; }
    This combine(This other) { return This(value + other.value); }
}
T combine<T : Base>(T left, T right)
{
    left.update(right);
    return left;
}
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { output[0] = combine(Value(20), Value(22)).value; }
