RWStructuredBuffer<uint> output;
interface Read { int read(float input); }
struct Value : Read {
    float read(int input, int extra = 7, int tail = 2) { return float(input + extra + tail) + 0.75; }
};
int invoke<T : Read>(T value, float input) { return value.read(input); }
interface StaticRead { static int read(float input); }
struct StaticValue : StaticRead {
    static int read(int input, int extra = 11) { return input + extra; }
};
int staticInvoke<T : StaticRead>() { return T.read(3.9); }
interface ReceiverRead { static int read(This value, float input); }
struct Receiver : ReceiverRead {
    int offset;
    int read(int input, int extra = 13) { return offset + input + extra; }
};
int receiverInvoke<T : ReceiverRead>(T value) { return T.read(value, 3.9); }
interface ZeroRead { int read(); }
struct ZeroValue : ZeroRead {
    int read(int extra = 31) { return extra; }
};
int zeroInvoke<T : ZeroRead>(T value) { return value.read(); }
interface DefaultRead { int read(float input = 3.9) { return 99; } }
struct OverrideValue : DefaultRead {
    override int read(int input, int extra = 37) { return input + extra; }
};
int defaultInvoke<T : DefaultRead>(T value) { return value.read(); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Value value;
    output[0] = uint(invoke(value, 3.9));
    output[1] = uint(invoke<Value>(value, 5.9));
    output[2] = uint(staticInvoke<StaticValue>());
    Receiver receiver = { 2 };
    output[3] = uint(receiverInvoke(receiver));
    ZeroValue zeroValue;
    output[4] = uint(zeroInvoke(zeroValue));
    OverrideValue overridden;
    output[5] = uint(defaultInvoke(overridden));
}
