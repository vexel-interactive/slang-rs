RWStructuredBuffer<uint> output;
static int value = 99;
static int serial = 0;
[ForceInline] int readValue() { return 5; }
struct Pair { int x; int y; };
struct State
{
    static int value = 3;
    static const int bias = 2;
    static const int computed = readValue();
    static int fromMethod = initial();
    static Pair pair = {4, 7};
    static int data[3] = {8, 9, 10};
    int field = bias;
    [ForceInline] static int initial() { return bias + 4; }
    [ForceInline] int read(int extra = this.bias) { return value + field + extra; }
    [ForceInline] void add(int extra) { value += extra; }
    [ForceInline] int shadow(int value) { return value + this.value; }
    [ForceInline] void change(inout uint x) { x += 2u; }
};
struct Effect
{
    static int value = 11;
    int field;
    [ForceInline] __init() { serial++; field = 1; }
};
struct A_B { static int c = 12; };
struct A { static int B_c = 13; };
typealias Alias = State;
static const int memberCopy = State.value;
[ForceInline] State make() { serial++; return {}; }
[ForceInline] int fromParameter(State s, int extra = s.bias) { return extra; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    State a = {}, b = {};
    const State immutable = {};
    output[0] = uint(a.read());
    immutable.add(4);
    output[1] = uint(b.value);
    output[2] = uint(Alias.value);
    output[3] = uint(a.shadow(20));
    State.pair.x += 2;
    output[4] = uint(b.pair.x + State.pair.y);
    State.data[1] += 3;
    output[5] = uint(a.data[1]);
    a.change(State.value);
    output[6] = uint(State.value);
    output[7] = uint(State.computed);
    output[8] = uint(State.fromMethod);
    output[9] = uint(memberCopy);
    output[10] = uint(Effect().value);
    output[11] = uint(make().value);
    Effect effects[2];
    int index = 0;
    output[12] = uint(effects[index++]::value);
    output[13] = uint(index);
    output[14] = uint(serial);
    output[15] = uint(A_B.c + A.B_c);
    output[16] = uint(fromParameter(a));
    output[17] = uint(value);
    output[18] = uint(a.field);
}
