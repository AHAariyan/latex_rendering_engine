import 'package:flutter/material.dart';
import 'package:mathcore_flutter/mathcore_flutter.dart';

void main() => runApp(const MaterialApp(home: Demo()));

class Demo extends StatefulWidget {
  const Demo({super.key});
  @override
  State<Demo> createState() => _DemoState();
}

class _DemoState extends State<Demo> {
  final controller = TextEditingController(text: r'\frac{a}{b} + \sqrt{x^2 + y^2}');
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('mathcore demo')),
      body: ListView(padding: const EdgeInsets.all(16), children: [
        TextField(controller: controller, onChanged: (_) => setState(() {})),
        const SizedBox(height: 12),
        MathText(controller.text, fontSize: 26),
        const Divider(),
        const MathText(r'x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}', fontSize: 22),
        const MathText(r'\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}', fontSize: 22),
        const MathText(r'f(x) = \begin{cases} x^2 & x \ge 0 \\ -x & \text{otherwise} \end{cases}', fontSize: 22),
      ]),
    );
  }
}
