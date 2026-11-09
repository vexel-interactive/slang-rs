T zero<T = int>() { return T(); }
T defaultValue<T = int>(T value = T()) { return value; }
V chain<T = int, U = T, V = U>() { return V(); }
struct Item { int value = 7; };
T item<T = Item>() { return T(); }
struct Constructed
{
    int value;
    __init() { value = 11; }
    __init(Constructed other) { value = other.value + 13; }
};
T copy<T>(T value) { return T(value); }
T optional<T = int>() { return T(); }
int optional() { return 9; }
int relay<T = int>() { return zero() + item().value; }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(zero());
    output[1] = uint(zero<float2>().y);
    output[2] = uint(defaultValue());
    output[3] = uint(chain());
    output[4] = uint(chain<float2>().x);
    output[5] = uint(item().value);
    output[6] = uint(optional());
    output[7] = uint(relay());
    output[8] = uint(Item().value);
    output[9] = uint(zero<Constructed>().value);
    Constructed value = Constructed();
    output[10] = uint(value.value);
    output[11] = uint(copy<Constructed>(value).value);
    output[12] = uint(Constructed(value).value);
    output[13] = uint(zero<Item[2]>()[1].value);
}
