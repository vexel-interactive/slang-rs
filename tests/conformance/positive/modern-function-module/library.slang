module library;

func hidden() -> int
{
    return 7;
}

public func adjusted(input : int = hidden()) -> int
{
    return input + 3;
}

public func copy<T>(input : T) -> T
{
    return input;
}
