RWStructuredBuffer<uint> output;
static int value = 99;
static int serial = 0;
int fixed(int x) { return 100; }
struct Counter
{
    int value;
    [ForceInline] int read() { return value; }
    [ForceInline] int readPair(int bias = 2) { return read() + this.read() + bias; }
    [ForceInline] [mutating] int add(int delta) { value += delta; return value; }
    [ForceInline] [mutating] int add(float delta) { value += int(delta) + 1; return value; }
    [ForceInline] int shadow(int value) { int result = value; return this.value + result; }
    [ForceInline] int snapshot(int changed) { return value * 10 + changed; }
    [ForceInline] int[2] pair() { return {value, value + 1}; }
    [ForceInline] static int fixed(int x = 2) { return x + 7; }
    [ForceInline] static int nested() { return fixed(3); }
    [ForceInline] int choose() { return value; }
    [ForceInline] static int choose(int x) { return x + 2; }
    [ForceInline] int prototype(int x);
    int prototype(int x) { return value + x; }
    [ForceInline] void readInto(out int x) { x = value + 1; }
    [ForceInline] [mutating] int change(inout uint x) { value += int(x); x += 2u; return value; }
};
struct Container
{
    Counter counter;
    [ForceInline] [mutating] int forward(int delta) { return counter.add(delta); }
};
struct Effect
{
    int value;
    [ForceInline] [mutating] void set(int x) { value = x; }
    [ForceInline] __init(int x) { serial++; set(x); }
    [ForceInline] static int fixed() { return 9; }
};
struct A_B { [ForceInline] static int c(int x) { return x + 1; } };
struct A { [ForceInline] static int B_c(int x) { return x + 2; } };
struct Clash
{
    int value;
    [ForceInline] int pick() { return value; }
    [ForceInline] static int pick(Clash x) { return x.value + 5; }
};
typealias Alias = Counter;
[ForceInline] int replace(inout Counter x, int n) { x.value = n; return n; }
static int global = Counter(7).readPair();
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Counter c = Counter(3);
    output[0] = uint(c.read());
    output[1] = uint(c.readPair());
    output[2] = uint(c.add(2));
    output[3] = uint(c.value);
    output[4] = uint(c.add(1.0));
    output[5] = uint(c.shadow(10));
    const Counter immutable = Counter(6);
    output[6] = uint(immutable.read());
    output[7] = uint(Counter(4).readPair());
    output[8] = uint(c.choose());
    output[9] = uint(c.choose(4));
    output[10] = uint(Counter.fixed());
    output[11] = uint(Counter.nested());
    output[12] = uint(global);
    c = Counter(3);
    output[13] = uint(c.snapshot(replace(c, 9)));
    output[14] = uint(c.value);
    c = Counter(3);
    output[15] = uint(c.add(c.value++));
    output[16] = uint(c.value);
    int values[2] = c.pair();
    output[17] = uint(values[0] + values[1]);
    Counter array[2] = {Counter(2), Counter(8)};
    int index = 0;
    output[18] = uint(array[index++].add(3));
    output[19] = uint(array[0].value);
    output[20] = uint(index);
    Container nested = {Counter(4)};
    output[21] = uint(nested.forward(2));
    output[22] = uint(nested.counter.value);
    output[23] = uint(Effect(3).fixed());
    output[24] = uint(serial);
    output[25] = uint(Alias.fixed(1));
    output[26] = uint(A_B.c(3));
    output[27] = uint(A.B_c(3));
    output[28] = uint(Clash(4).pick());
    output[29] = uint(Clash.pick(Clash(4)));
    output[30] = uint(c.prototype(2));
    int result;
    c.readInto(result);
    output[31] = uint(result);
    int viewed = 3;
    output[32] = uint(c.change(viewed));
    output[33] = uint(viewed);
    output[34] = uint(c.value);
}
