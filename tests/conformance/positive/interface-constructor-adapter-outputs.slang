interface IView {
    __init(inout int input);
}
struct View : IView {
    int value;

    __init(inout uint input) {
        input += 2;
        value = int(input);
    }
}

interface IRead {
    __init(inout int input);
}
struct Read : IRead {
    int value;

    __init(float input) { value = int(input) + 3; }
}

interface IWrite {
    __init(inout int input);
}
struct Write : IWrite {
    int value;

    __init(out int input) {
        input = 9;
        value = 10;
    }
}

T fromView<T : IView>(inout int input) { return T(input); }
T fromRead<T : IRead>(inout int input) { return T(input); }
T fromWrite<T : IWrite>(inout int input) { return T(input); }

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    int values[2] = { 4, 8 };
    int index = 0;
    View view = fromView<View>(values[index++]);
    output[0] = view.value;
    output[1] = values[0];
    output[2] = index;
    Read read = fromRead<Read>(values[1]);
    output[3] = read.value;
    output[4] = values[1];
    Write write = fromWrite<Write>(values[1]);
    output[5] = write.value;
    output[6] = values[1];
}
