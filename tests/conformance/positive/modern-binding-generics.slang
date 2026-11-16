struct Box<T> {
    var value : T;
    static var shared = 2;

    __init(T input) { value = input; }
    T read() {
        let copy = value;
        return copy;
    }

    U convert<U>(U input) {
        var copy = input;
        return copy;
    }
}

T identity<T>(T input) {
    let copy = input;
    return copy;
}

int[2] pair() { return { 3, 4 }; }

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var item = Box<int>(5);
    let copy = identity(item);
    var values = identity(pair());
    let vector = item.convert(float2(6, 7));
    Box<int>.shared += 1;
    output[0] = uint(copy.read());
    output[1] = uint(values[0] + values[1]);
    output[2] = uint(vector.y);
    output[3] = uint(Box<int>.shared);
    output[4] = uint(Box<float>.shared);
}
