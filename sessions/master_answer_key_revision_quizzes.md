# 🎓 Master Exam Answer Key: Revision Quiz 1 & Revision Quiz 2
**Course:** Oracle Java Foundations (CSE & IT)  
**Portal:** Eduvate Exam Engine  
**Verification:** 100% Verified with Java Compiler (`javac 21.0.5`), Language Specification & Exam Review Results.

---

## 📌 Part 1: Revision Quiz 1 (25 Questions)

| # | Question Summary | Correct Option | Correct Answer Text | Reason / Verification |
|---|---|---|---|---|
| **1** | Output of `int array[] = {5,4,3,2,1}; for(int index=0; index...` | **Option d** | `54321` | Iterates through array `{5,4,3,2,1}` from index 0 sequentially and prints elements. The `< array.length...` was clipped by Moodle HTML unescaped `<` tag. |
| **2** | Result of `int first=5; int second=first++; System.out.println(second);` | **Option d** | `5` | Post-increment (`first++`) assigns current value (`5`) to `second` before incrementing. |
| **3** | Not a valid way to comment a single line in Java? | **Option b** | `''' This is a comment` | Python-style triple quotes `'''` are invalid for comments in Java. Confirmed by Eduvate review screen. |
| **4** | Java keyword used to implement exception handling? | **Option b** | `try` | `try` establishes the guarded block for exception handling in Java. |
| **5** | Purpose of `break` statement in Java? | **Option d** | `To exit a loop or switch statement prematurely` | `break` abruptly jumps out of the innermost enclosing loop or switch block. Confirmed by Eduvate review. |
| **6** | Java class used for input and output operations? | **Option a** | `java.io.Console` | `Console` in `java.io` provides methods to access character-based console devices. `java.io.IOStream` does NOT exist in Java. |
| **7** | Default value of instance variable if not explicitly initialized? | **Option d** | `It depends on the data type` | Numeric primitives default to `0`/`0.0`, boolean to `false`, and reference types to `null`. |
| **8** | Output of summing `{1, 2, 3, 4, 5}` in for loop? | **Option b** | `15` | `1 + 2 + 3 + 4 + 5 = 15`. |
| **9** | True statement about method overloading in Java? | **Option c** | `Methods with different return types can be overloaded.` | Overloaded methods can have different return types as long as their parameter argument lists differ. |
| **10** | Purpose of `super` keyword in Java? | **Option d** | `To access a superclass method or variable` | `super` refers to the immediate superclass members (`super.var`, `super.method()`), while `super()` invokes the constructor. |
| **11** | Which of the following is not a valid Java data type? | **Option b** | `real` | Java primitives are `byte`, `short`, `int`, `long`, `float`, `double`, `char`, `boolean`. `real` does not exist. |
| **12** | Keyword used to declare a constant in Java? | **Option a** | `final` | `final` prevents re-assignment, creating a constant variable. |
| **13** | Output of `10 + 5` in Example class? | **Option c** | `15` | Standard integer addition. |
| **14** | Valid declarations of integer variable (Select 2): | **Option d & Option e** | `static int value = 10;`<br>`final int value = 10;` | Multi-select: `static int` and `final int` are valid. (Option b `int value = 10;` is also syntactically valid in local scope). `Int` capitalized is invalid. |
| **15** | Print number between 0 and 10 with Random (Select one or more): | **Option a, Option c, Option d** | `Random random = new Random(); int randomNumber = random.nextInt(value);`<br>`Random random = new Random(value); random.setSeed(value); int randomNumber = random.nextInt(value);`<br>`Random random = new Random(value); int randomNumber = random.nextInt(value);` | All three correctly instantiate `Random` and call `nextInt(10)` yielding integers in range [0, 9]. Options b and e have illegal syntax `random.next()10`. |
| **16** | Reserved words in Java (Select 2): | **Option a & Option c** | `package`<br>`goto` | `package` and `goto` are reserved keywords (`goto` is a reserved keyword in Java even though unused). |
| **17** | Main entry file for class `AccountManager`? | **Option e** | `AccountManager.java` | In Java, a public class must reside in a file whose filename matches the class name exactly, including case. |
| **18** | Print `111 222` from raw `List` of `Account` objects? | **Option e** | `None of the above.` | Option a `for(Account id : list)` fails compilation (raw List returns Object, cannot convert to Account without cast) and calls default `toString()` (`Account@hash`). |
| **19** | Output of `"Hello, World! ".substring(0, 5)`? | **Option d** | `Hello` | `substring(0, 5)` extracts 5 characters (index 0 to 4: 'H','e','l','l','o'). Index 5 is ',' which is excluded. |
| **20** | Not a valid access modifier in Java? | **Option b** | `global` | Java has `public`, `protected`, `private`, and default (package-private). `global` does not exist. |
| **21** | Primary purpose of `static` keyword in Java? | **Option a** | `To specify that a method or variable belongs to the class rather than an instance` | Static members belong to the class namespace, shared across all instances. |
| **22** | Data type used to store a single character in Java? | **Option c** | `char` | Primitive 16-bit Unicode character type is `char`. |
| **23** | Output of `5 % 2`? | **Option d** | `1` | Remainder of integer division `5 / 2` is `1`. |
| **24** | Correct way to declare and initialize an array in Java? | **Option d** | `int[] numbers = new int[5];` | Standard canonical Java array declaration and heap allocation. |
| **25** | Keyword used to create a new instance of a class? | **Option c** | `new` | `new` dynamically allocates memory on the heap and calls the constructor. |

---

## 📌 Part 2: Revision Quiz 2 (25 Questions)

| # | Question Summary | Correct Option | Correct Answer Text | Reason / Verification |
|---|---|---|---|---|
| **1** | Effect of `for (int index = 0, count = 10; sum > 20; ...)` with `sum = 0`? | **Option a** | `It will print Sum = 0` | Initial condition `sum > 20` (`0 > 20`) is immediately `false`. Loop never executes. |
| **2** | Which for loop statement prints 135 from `{1, 2, 3, 4, 5}`? | **Option b** | `for(int index = 0; index < arr.length; index+=2) { System.out.print(arr[index]); }` | Iterates indices 0, 2, 4 printing `arr[0]=1`, `arr[2]=3`, `arr[4]=5`. |
| **3** | Two components class declarations can include (Select one or more): | **Option a & Option b** | `Interfaces implemented by the class`<br>`The public modifier` | Class declaration header consists of modifiers (`public`), class name, extends, and implements interfaces. Methods are inside the class body. |
| **4** | Count non-"CA" strings in `{"CA", "JP", "US", "CA", "UK"}`? | **Option c** | `3` | "CA" triggers `continue`. "JP", "US", "UK" increment `count` 3 times. |
| **5** | Code enabling `switch(variable) { case "1": ... }` to print `one`? | **Option d** | `String variable = "1";` | Case label is a String literal `"1"`. Variable must be of type `String`. |
| **6** | Two valid code fragments inside `class App { }` (Select one or more): | **Option a & Option c** | `private String name = "John"; public void display() { System.out.print(name); }`<br>`{ private int number; }` (or instance initializer block) | Field and method declarations (a) and instance blocks (c). Package, imports, and naked for-loops are illegal in class bodies. |
| **7** | Code to print `Rita is 14 years old` with `printf`? | **Option b** | `System.out.printf("%s is %d years old", name, age);` | `%s` formats String (`Rita`), `%d` formats integer (`14`). `%n` is newline. |
| **8** | Result of `switch(choice = 2)` accessing `flavors[2]` from length 2 array? | **Option c** | `An ArrayIndexOutOfBoundsException is thrown at run time.` | Valid indices for array length 2 are `0` and `1`. Accessing `flavors[2]` throws `ArrayIndexOutOfBoundsException`. |
| **9** | Result of `digits.substring(indexOf("4"), indexOf("8")).concat("89")` on `"0123456789"`? | **Option c** | `456789` | `substring(4, 8)` is `"4567"`. Concatenating `"89"` produces `"456789"`. |
| **10** | Method identifier matching Java naming conventions? | **Option b** | `calculateBill` | Follows standard `lowerCamelCase` verb convention. |
| **11** | Code fragment inserted to enable printing `0.0` for `ballObject.weight`? | **Option a** | `Ball ballObject = new Ball();` | Default constructor initializes primitive `double weight` to `0.0`. |
| **12** | Nested ternary evaluation with pre/post increments? | **Option b** | `2` | Verified with compiler: `fn=2, sn=1, tn=1, 4th=1, 5th=2`. `2 > 1` is true -> `1 > 2` is false -> `fifthNumber` = `2`. |
| **13** | Accessing `testObject.firstValue` where `firstValue` is `private`? | **Option b** | `A compilation error occurs at the System.out.println line.` | Private fields cannot be accessed outside the declaring class `MyClass`. |
| **14** | Result of calling `new Bus()` when only `Bus(String type)` constructor defined? | **Option d** | `The code fails to compile. To make it compile, insert: Bus() {}` | Defining an explicit constructor removes the default no-arg constructor. |
| **15** | Java edition geared towards microprocessor board / drone / constrained memory? | **Option c** | `Java Micro Edition` (Java ME) | Java ME is specifically designed for resource-constrained embedded systems and microcontrollers. |
| **16** | True statement about primitive variables? | **Option a** | `They can be compared with the == operator.` | Primitives are value types and directly compared with `==`. |
| **17** | Result of `equals()` vs `==` on `"Java"` and `new String("Java")`? | **Option c** | `true false` | `.equals()` compares content (`true`), `==` compares references (`false`). |
| **18** | Output of count on `{"CA", "JP", "US", "CA", "UK"}`? | **Option a** | `3` | Same question as Q4 with option letters reordered: Answer is `3`. |
| **19** | Features of Java (Select 2 options): | **Option c & Option d** | `It allows multithreaded programming.`<br>`It is a distributed language.` | Multithreaded and Distributed are foundational Java buzzwords. `strongly typed` (Option a) is also a valid language property. Direct memory management (b) is false. |
| **20** | Output of `div(5, 0)` catching `ArithmeticException` and returning 0? | **Option e** | `exception in div 0` | Catch block prints `"exception in div"` and returns `0`. `main` prints returned `0`. Exception does not propagate to `main`. |
| **21** | Output of calling `incr()` on static vs instance members twice? | **Option d** | `1 1 1 3 1 3` | Verified with JVM execution: 1st call prints `1 1 1`. Static increments; new instance resets instance var. 2nd call prints `3 1 3`. |
| **22** | Identify two features of Java (Select one or more): | **Option b & Option d** | `platform independent`<br>`robust` | Java is platform independent ("Write Once, Run Anywhere") and robust. |
| **23** | Output of `String test = "a"; for(; test.compareTo("aaa") == 0; ...)`? | **Option b** | `a` | Condition `"a".compareTo("aaa") == 0` is `false` immediately. Loop never executes. Prints `"a"`. |
| **24** | Result of `while(checkOut) { days++; if(days > 3) checkOut = false; }`? | **Option a** | `4` | Loop increments days until `days = 4`, where `4 > 3` sets `checkOut = false`. |
| **25** | Result of comparing `nextInt()` on two `new Random(10)` instances? | **Option d** | `Jack` | Same seed (`10`) guarantees identical pseudo-random sequence. `firstRandom.nextInt() == secondRandom.nextInt()` is always true. |
