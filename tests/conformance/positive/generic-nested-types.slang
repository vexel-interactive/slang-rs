struct Outer<T>
{
    struct Item<U = T> { T first; U second; };
    struct Plain { T value; };
    typealias Shape<U = T> = vector<U, 2>;
};
struct Owner { struct Item<T> { T value; }; };
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Outer<int>.Item<float> first = {2, 3};
    Outer<float>.Item<int> second = Outer<float>.Item<int>(4, 5);
    Outer<int>.Item<> third = {6, 7};
    Outer<int>.Plain fourth = Outer<int>.Plain(8);
    Owner.Item<int> fifth = Owner.Item<int>(9);
    Outer<int>.Shape<> shape = {10, 11};
    output[0] = uint(first.first + first.second);
    output[1] = uint(second.first + second.second);
    output[2] = uint(third.first + third.second);
    output[3] = uint(fourth.value);
    output[4] = uint(fifth.value);
    output[5] = uint(shape.y);
}
