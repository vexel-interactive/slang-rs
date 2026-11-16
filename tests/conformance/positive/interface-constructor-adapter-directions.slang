interface ICopyOut { __init(const int input); }
struct CopyOut : ICopyOut {
    int value;

    __init(out uint input) {
        input = 7;
        value = 8;
    }
}

interface ICopyInout { __init(const int input); }
struct CopyInout : ICopyInout {
    int value;

    __init(inout uint input) {
        input += 2;
        value = int(input);
    }
}

interface IOutInput { __init(out int input); }
struct OutInput : IOutInput {
    int value;

    // The required output stays undefined; only the constructed value is used.
    __init(float input) { value = 11; }
}

interface IOutView { __init(out int input); }
struct OutView : IOutView {
    int value;

    __init(out uint input) {
        input = 7;
        value = 8;
    }
}

interface IOutInout { __init(out int input); }
struct OutInout : IOutInout {
    int value;

    __init(inout uint input) {
        input = 7;
        value = 8;
    }
}

T copyOut<T : ICopyOut>(inout int input) { return T(input); }
T copyInout<T : ICopyInout>(inout int input) { return T(input); }
T outInput<T : IOutInput>(out int input) { return T(input); }
T outView<T : IOutView>(out int input) { return T(input); }
T outInout<T : IOutInout>(out int input) { return T(input); }

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    int input = 4;
    output[0] = copyOut<CopyOut>(input).value;
    output[1] = input;
    output[2] = copyInout<CopyInout>(input).value;
    output[3] = input;

    int unused;
    output[4] = outInput<OutInput>(unused).value;
    output[5] = outView<OutView>(input).value;
    output[6] = input;
    output[7] = outInout<OutInout>(input).value;
    output[8] = input;
}
