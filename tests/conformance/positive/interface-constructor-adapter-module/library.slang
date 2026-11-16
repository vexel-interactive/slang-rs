public interface IBuild {
    __init(int input = 4);
}

int extra() { return 3; }

public struct Value : IBuild {
    public int value;

    internal __init(float input, int tail = extra()) {
        value = int(input) + tail;
    }
}

public T make<T : IBuild>(int input) { return T(input); }
public T makeDefault<T : IBuild>() { return T(); }
