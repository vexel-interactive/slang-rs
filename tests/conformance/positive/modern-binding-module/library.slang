module library;

public struct Item {
    public var value = 5;
    public var bias : int = 2;
    public static let extra = helper();

    public int read() { return value + bias + extra; }
}

int helper() { return 3; }
public static let value = Item(5, 2);
public T copy<T>(T input) {
    let result = input;
    return result;
}
