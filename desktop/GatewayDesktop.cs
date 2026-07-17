using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Net;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Effects;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;
using System.Windows.Threading;
using WinForms = System.Windows.Forms;
using SD = System.Drawing;
using Application = System.Windows.Application;
using Brush = System.Windows.Media.Brush;
using Clipboard = System.Windows.Clipboard;
using Color = System.Windows.Media.Color;
using Cursors = System.Windows.Input.Cursors;
using FontFamily = System.Windows.Media.FontFamily;
using MessageBox = System.Windows.MessageBox;
using Path = System.IO.Path;
using SolidColorBrush = System.Windows.Media.SolidColorBrush;
using WpfButton = System.Windows.Controls.Button;

namespace CodexChatGateway.Desktop
{
    internal static class Program
    {
        private const string MutexName = @"Local\CodexChatGateway.Desktop.Singleton";
        private const string ShowEventName = @"Local\CodexChatGateway.Desktop.Show";

        [STAThread]
        private static int Main(string[] args)
        {
            if (args != null && args.Any(a => string.Equals(a, "--selftest", StringComparison.OrdinalIgnoreCase)))
                return SelfTest.Run();

            bool created;
            Mutex mutex = new Mutex(true, MutexName, out created);
            if (!created)
            {
                try { EventWaitHandle.OpenExisting(ShowEventName).Set(); } catch { }
                return 0;
            }

            EventWaitHandle showEvent = new EventWaitHandle(false, EventResetMode.AutoReset, ShowEventName);
            MainWindow window = null;
            Thread listener = new Thread(new ThreadStart(delegate
            {
                while (true)
                {
                    try
                    {
                        showEvent.WaitOne();
                        Application app = Application.Current;
                        if (app == null || window == null) continue;
                        app.Dispatcher.BeginInvoke(new Action(delegate { window.ShowFromTray(); }));
                    }
                    catch { }
                }
            }));
            listener.IsBackground = true;
            listener.Start();

            Application application = new Application();
            window = new MainWindow();
            application.Run(window);
            GC.KeepAlive(mutex);
            return 0;
        }
    }

    internal static class Theme
    {
        public static readonly Color Ink = Color.FromRgb(0x00, 0x00, 0x00);
        public static readonly Color Deep = Color.FromRgb(0x05, 0x08, 0x07);
        public static readonly Color Panel = Color.FromRgb(0x0E, 0x15, 0x13);
        public static readonly Color PanelSoft = Color.FromRgb(0x13, 0x1D, 0x1A);
        public static readonly Color CardLine = Color.FromRgb(0x1E, 0x2E, 0x2A);
        public static readonly Color Field = Color.FromRgb(0x07, 0x0C, 0x0B);
        public static readonly Color Muted = Color.FromRgb(0x9D, 0xB8, 0xCF);
        public static readonly Color Paper = Color.FromRgb(0xF8, 0xF4, 0xEE);
        public static readonly Color Mint = Color.FromRgb(0x00, 0xF5, 0xD4);
        public static readonly Color MintDark = Color.FromRgb(0x00, 0xC4, 0xA9);
        public static readonly Color Gold = Color.FromRgb(0xF4, 0xD2, 0x8A);
        public static readonly Color Ice = Color.FromRgb(0x7F, 0xD8, 0xFF);
        public static readonly Color Danger = Color.FromRgb(0xFF, 0x53, 0x67);
        public static readonly Color Selection = Color.FromRgb(0x16, 0x30, 0x2B);

        public static readonly FontFamily Ui = new FontFamily("Segoe UI, Microsoft YaHei UI");
        public static readonly FontFamily Mono = new FontFamily("JetBrains Mono, Consolas");

        public static SolidColorBrush B(Color color)
        {
            SolidColorBrush brush = new SolidColorBrush(color);
            brush.Freeze();
            return brush;
        }

        public static SolidColorBrush BA(Color color, byte alpha)
        {
            Color c = Color.FromArgb(alpha, color.R, color.G, color.B);
            return B(c);
        }

        public static readonly SolidColorBrush PaperBrush = B(Paper);
        public static readonly SolidColorBrush MutedBrush = B(Muted);
        public static readonly SolidColorBrush MintBrush = B(Mint);
        public static readonly SolidColorBrush IceBrush = B(Ice);
        public static readonly SolidColorBrush GoldBrush = B(Gold);
        public static readonly SolidColorBrush DangerBrush = B(Danger);
        public static readonly SolidColorBrush LineBrush = B(CardLine);
        public static readonly SolidColorBrush Glass = BA(Panel, 0xE6);
        public static readonly SolidColorBrush FieldBrush = B(Field);
    }

    internal enum ButtonKind { Primary, Ghost, Danger }

    internal static class Ui
    {
        public static TextBlock Section(string text)
        {
            return new TextBlock
            {
                Text = text,
                Foreground = Theme.MutedBrush,
                FontFamily = Theme.Ui,
                FontSize = 11,
                FontWeight = FontWeights.SemiBold,
                Margin = new Thickness(0, 0, 0, 10)
            };
        }

        public static TextBlock Label(string text, double size, Brush brush)
        {
            return new TextBlock
            {
                Text = text,
                Foreground = brush,
                FontFamily = Theme.Ui,
                FontSize = size
            };
        }

        public static Border Card(UIElement child)
        {
            return new Border
            {
                Background = Theme.Glass,
                BorderBrush = Theme.LineBrush,
                BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(12),
                SnapsToDevicePixels = true,
                Child = child,
                Effect = new DropShadowEffect { Color = Colors.Black, BlurRadius = 26, ShadowDepth = 0, Opacity = 0.4, RenderingBias = RenderingBias.Performance }
            };
        }

        public static WpfButton Button(string text, ButtonKind kind)
        {
            WpfButton button = new WpfButton
            {
                Content = text,
                Cursor = Cursors.Hand,
                FontFamily = Theme.Ui,
                FontSize = 12.5,
                FontWeight = FontWeights.SemiBold,
                Padding = new Thickness(12, 9, 12, 9)
            };
            ApplyKind(button, kind);
            return button;
        }

        public static WpfButton WideButton(string title, string subtitle, ButtonKind kind)
        {
            StackPanel content = new StackPanel();
            content.Children.Add(new TextBlock { Text = title, Foreground = Theme.PaperBrush, FontFamily = Theme.Ui, FontSize = 12.5, FontWeight = FontWeights.SemiBold });
            content.Children.Add(new TextBlock { Text = subtitle, Foreground = Theme.MutedBrush, FontFamily = Theme.Ui, FontSize = 10.5, Margin = new Thickness(0, 2, 0, 0) });
            WpfButton button = new WpfButton
            {
                Content = content,
                Cursor = Cursors.Hand,
                HorizontalContentAlignment = HorizontalAlignment.Left,
                Padding = new Thickness(14, 9, 14, 9)
            };
            ApplyKind(button, kind);
            return button;
        }

        private static void ApplyKind(WpfButton button, ButtonKind kind)
        {
            SolidColorBrush bg, hover, pressed, line, fg;
            switch (kind)
            {
                case ButtonKind.Primary:
                    bg = Theme.B(Theme.Mint); hover = Theme.B(Color.FromRgb(0x9C, 0xFF, 0xDF)); pressed = Theme.B(Theme.MintDark);
                    line = Theme.B(Theme.Mint); fg = Theme.B(Theme.Deep);
                    break;
                case ButtonKind.Danger:
                    bg = Theme.B(Theme.PanelSoft); hover = Theme.B(Color.FromRgb(0x2A, 0x18, 0x1C)); pressed = Theme.B(Theme.Panel);
                    line = Theme.B(Color.FromRgb(0x5A, 0x2A, 0x33)); fg = Theme.B(Color.FromRgb(0xFF, 0x7A, 0x90));
                    break;
                default:
                    bg = Theme.B(Theme.PanelSoft); hover = Theme.B(Color.FromRgb(0x1B, 0x28, 0x24)); pressed = Theme.B(Theme.Panel);
                    line = Theme.LineBrush; fg = Theme.PaperBrush;
                    break;
            }
            button.Foreground = fg;
            button.Background = bg;
            button.BorderBrush = line;
            button.Template = ButtonTemplate(bg, hover, pressed, line);
        }

        private static ControlTemplate ButtonTemplate(Brush bg, Brush hover, Brush pressed, Brush line)
        {
            ControlTemplate template = new ControlTemplate(typeof(WpfButton));
            FrameworkElementFactory border = new FrameworkElementFactory(typeof(Border), "bd");
            border.SetValue(Border.BackgroundProperty, bg);
            border.SetValue(Border.BorderBrushProperty, line);
            border.SetValue(Border.BorderThicknessProperty, new Thickness(1));
            border.SetValue(Border.CornerRadiusProperty, new CornerRadius(7));
            FrameworkElementFactory content = new FrameworkElementFactory(typeof(ContentPresenter));
            content.SetValue(ContentPresenter.HorizontalAlignmentProperty, new TemplateBindingExtension(Control.HorizontalContentAlignmentProperty));
            content.SetValue(ContentPresenter.VerticalAlignmentProperty, new TemplateBindingExtension(Control.VerticalContentAlignmentProperty));
            content.SetValue(ContentPresenter.MarginProperty, new TemplateBindingExtension(Control.PaddingProperty));
            border.AppendChild(content);
            template.VisualTree = border;

            Trigger isHover = new Trigger { Property = UIElement.IsMouseOverProperty, Value = true };
            isHover.Setters.Add(new Setter(Border.BackgroundProperty, hover, "bd"));
            template.Triggers.Add(isHover);
            Trigger isPressed = new Trigger { Property = WpfButton.IsPressedProperty, Value = true };
            isPressed.Setters.Add(new Setter(Border.BackgroundProperty, pressed, "bd"));
            template.Triggers.Add(isPressed);
            Trigger isDisabled = new Trigger { Property = UIElement.IsEnabledProperty, Value = false };
            isDisabled.Setters.Add(new Setter(UIElement.OpacityProperty, 0.4));
            template.Triggers.Add(isDisabled);
            return template;
        }

        public static ControlTemplate InputTemplate(Type targetType)
        {
            ControlTemplate template = new ControlTemplate(targetType);
            FrameworkElementFactory border = new FrameworkElementFactory(typeof(Border), "bd");
            border.SetValue(Border.BackgroundProperty, new TemplateBindingExtension(Control.BackgroundProperty));
            border.SetValue(Border.BorderBrushProperty, new TemplateBindingExtension(Control.BorderBrushProperty));
            border.SetValue(Border.BorderThicknessProperty, new TemplateBindingExtension(Control.BorderThicknessProperty));
            border.SetValue(Border.CornerRadiusProperty, new CornerRadius(6));
            FrameworkElementFactory host = new FrameworkElementFactory(typeof(ScrollViewer), "PART_ContentHost");
            border.AppendChild(host);
            template.VisualTree = border;
            Trigger focus = new Trigger { Property = UIElement.IsKeyboardFocusWithinProperty, Value = true };
            focus.Setters.Add(new Setter(Border.BorderBrushProperty, Theme.MintBrush, "bd"));
            template.Triggers.Add(focus);
            return template;
        }

        public static TextBox Input()
        {
            return new TextBox
            {
                Background = Theme.FieldBrush,
                Foreground = Theme.PaperBrush,
                BorderBrush = Theme.LineBrush,
                BorderThickness = new Thickness(1),
                Padding = new Thickness(10, 8, 10, 8),
                FontFamily = Theme.Ui,
                FontSize = 12.5,
                CaretBrush = Theme.MintBrush,
                Template = InputTemplate(typeof(TextBox)),
                VerticalContentAlignment = VerticalAlignment.Center
            };
        }

        public static PasswordBox PasswordInput()
        {
            return new PasswordBox
            {
                Background = Theme.FieldBrush,
                Foreground = Theme.PaperBrush,
                BorderBrush = Theme.LineBrush,
                BorderThickness = new Thickness(1),
                Padding = new Thickness(10, 8, 10, 8),
                FontFamily = Theme.Ui,
                FontSize = 12.5,
                CaretBrush = Theme.MintBrush,
                Template = InputTemplate(typeof(PasswordBox)),
                VerticalContentAlignment = VerticalAlignment.Center
            };
        }
    }

    internal sealed class ParticleField : FrameworkElement
    {
        private sealed class Particle { public double X, Y, Vx, Vy, R, A; }

        private const int ParticleCount = 70;
        private const double LinkDistance = 115;
        private const double FrameMs = 28;
        private const int AlphaBuckets = 8;

        private readonly Particle[] particles = new Particle[ParticleCount];
        private readonly Random random = new Random();
        private readonly Brush[][] dotBrushes;
        private readonly Pen[][] linePens;
        private Point mouse = new Point(-10000, -10000);
        private bool animating;
        private bool running;
        private double lastMs;

        public ParticleField()
        {
            dotBrushes = new Brush[2][];
            linePens = new Pen[2][];
            Color[] bases = { Color.FromRgb(0x6E, 0x82, 0x96), Theme.Mint };
            for (int c = 0; c < 2; c++)
            {
                dotBrushes[c] = new Brush[AlphaBuckets];
                linePens[c] = new Pen[AlphaBuckets];
                for (int i = 0; i < AlphaBuckets; i++)
                {
                    double dotAlpha = 0.25 + 0.65 * (i + 1) / AlphaBuckets;
                    RadialGradientBrush dot = new RadialGradientBrush();
                    dot.GradientStops.Add(new GradientStop(Color.FromArgb((byte)(dotAlpha * 255), bases[c].R, bases[c].G, bases[c].B), 0.0));
                    dot.GradientStops.Add(new GradientStop(Color.FromArgb(0, bases[c].R, bases[c].G, bases[c].B), 1.0));
                    dot.Freeze();
                    dotBrushes[c][i] = dot;

                    double lineAlpha = 0.28 * (i + 1) / AlphaBuckets;
                    SolidColorBrush lineBrush = new SolidColorBrush(Color.FromArgb((byte)(lineAlpha * 255), bases[c].R, bases[c].G, bases[c].B));
                    lineBrush.Freeze();
                    Pen pen = new Pen(lineBrush, 1.0);
                    pen.Freeze();
                    linePens[c][i] = pen;
                }
            }
            for (int i = 0; i < ParticleCount; i++) particles[i] = Spawn();
            IsHitTestVisible = false;
        }

        public bool Running
        {
            get { return running; }
            set { running = value; }
        }

        public void SetMouse(Point point) { mouse = point; }

        public void Start()
        {
            if (animating) return;
            animating = true;
            CompositionTarget.Rendering += OnFrame;
        }

        public void Stop()
        {
            animating = false;
            CompositionTarget.Rendering -= OnFrame;
        }

        private Particle Spawn()
        {
            double speed = 6 + random.NextDouble() * 14;
            double angle = random.NextDouble() * Math.PI * 2;
            return new Particle
            {
                X = random.NextDouble() * Math.Max(1, ActualWidth),
                Y = random.NextDouble() * Math.Max(1, ActualHeight),
                Vx = Math.Cos(angle) * speed,
                Vy = Math.Sin(angle) * speed,
                R = 1.2 + random.NextDouble() * 2.2,
                A = 0.3 + random.NextDouble() * 0.7
            };
        }

        private void OnFrame(object sender, EventArgs e)
        {
            double ms = RenderingEventArgsNow();
            if (ms - lastMs < FrameMs) return;
            double dt = Math.Min(0.06, lastMs <= 0 ? 0.016 : (ms - lastMs) / 1000.0);
            lastMs = ms;
            double w = ActualWidth, h = ActualHeight;
            if (w < 10 || h < 10) return;
            foreach (Particle p in particles)
            {
                p.X += p.Vx * dt;
                p.Y += p.Vy * dt;
                double dx = p.X - mouse.X, dy = p.Y - mouse.Y;
                double d2 = dx * dx + dy * dy;
                if (d2 < 140 * 140 && d2 > 0.01)
                {
                    double d = Math.Sqrt(d2);
                    double push = (140 - d) / 140 * 110 * dt;
                    p.X += dx / d * push;
                    p.Y += dy / d * push;
                }
                if (p.X < -12) p.X = w + 10; else if (p.X > w + 12) p.X = -10;
                if (p.Y < -12) p.Y = h + 10; else if (p.Y > h + 12) p.Y = -10;
            }
            InvalidateVisual();
        }

        private static double RenderingEventArgsNow()
        {
            return TimeSpan.FromTicks(Environment.TickCount * (long)10000).TotalMilliseconds;
        }

        protected override void OnRender(DrawingContext dc)
        {
            int c = running ? 1 : 0;
            double link2 = LinkDistance * LinkDistance;
            for (int i = 0; i < ParticleCount; i++)
            {
                Particle a = particles[i];
                for (int j = i + 1; j < ParticleCount; j++)
                {
                    Particle b = particles[j];
                    double dx = a.X - b.X, dy = a.Y - b.Y;
                    double d2 = dx * dx + dy * dy;
                    if (d2 >= link2 || d2 < 1) continue;
                    double strength = 1 - Math.Sqrt(d2) / LinkDistance;
                    int bucket = Math.Min(AlphaBuckets - 1, (int)(strength * AlphaBuckets));
                    dc.DrawLine(linePens[c][bucket], new Point(a.X, a.Y), new Point(b.X, b.Y));
                }
            }
            foreach (Particle p in particles)
            {
                int bucket = Math.Min(AlphaBuckets - 1, (int)(p.A * AlphaBuckets));
                double radius = p.R * 2.2;
                dc.DrawEllipse(dotBrushes[c][bucket], null, new Point(p.X, p.Y), radius, radius);
            }
        }
    }

    internal sealed class ModelStore
    {
        public int version { get; set; }
        public string default_id { get; set; }
        public List<ModelProfile> profiles { get; set; }
    }

    internal sealed class ModelProfile
    {
        public string id { get; set; }
        public string name { get; set; }
        public string base_url { get; set; }
        public string api_key { get; set; }
        public string model_id { get; set; }
        public string litellm_model { get; set; }
    }

    internal sealed class GatewayState
    {
        public int pid { get; set; }
        public string executable { get; set; }
        public string runner { get; set; }
        public string endpoint { get; set; }
        public string model { get; set; }
        public string started_at { get; set; }
    }

    internal sealed class ModelRow
    {
        public string Id { get; set; }
        public string Name { get; set; }
        public string ModelId { get; set; }
        public string Adapter { get; set; }
        public string BaseUrl { get; set; }
        public bool IsDefault { get; set; }
    }

    internal static class Store
    {
        private static readonly JavaScriptSerializer Json = CreateSerializer();

        private static JavaScriptSerializer CreateSerializer()
        {
            JavaScriptSerializer serializer = new JavaScriptSerializer();
            serializer.MaxJsonLength = int.MaxValue;
            return serializer;
        }

        public static JavaScriptSerializer JsonInstance { get { return Json; } }

        public static string FindProjectRoot(string start)
        {
            DirectoryInfo current = new DirectoryInfo(start);
            for (int i = 0; i < 4 && current != null; i++, current = current.Parent)
            {
                if (File.Exists(Path.Combine(current.FullName, "config.yaml")) && Directory.Exists(Path.Combine(current.FullName, "scripts")))
                    return current.FullName;
            }
            return Path.GetFullPath(start);
        }

        public static string StorePath(string root) { return Path.Combine(root, ".gateway", "models.json"); }

        public static ModelStore ReadStore(string root)
        {
            string path = StorePath(root);
            if (!File.Exists(path)) return EmptyStore();
            ModelStore store = Json.Deserialize<ModelStore>(File.ReadAllText(path, Encoding.UTF8));
            if (store == null) return EmptyStore();
            if (store.profiles == null) store.profiles = new List<ModelProfile>();
            return store;
        }

        public static ModelStore EmptyStore()
        {
            return new ModelStore { version = 1, default_id = "", profiles = new List<ModelProfile>() };
        }

        public static void SaveStore(string root, ModelStore store)
        {
            string path = StorePath(root);
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            string temp = path + ".desktop.tmp";
            File.WriteAllText(temp, Json.Serialize(store), new UTF8Encoding(false));
            if (File.Exists(path)) File.Replace(temp, path, path + ".bak", true);
            else File.Move(temp, path);
        }

        public static string LiteLLMModel(string baseUrl, string modelId)
        {
            if (baseUrl != null && baseUrl.IndexOf("deepseek", StringComparison.OrdinalIgnoreCase) >= 0)
                return modelId.StartsWith("deepseek/", StringComparison.OrdinalIgnoreCase) ? modelId : "deepseek/" + modelId;
            return modelId.StartsWith("openai/", StringComparison.OrdinalIgnoreCase) ? modelId : "openai/" + modelId;
        }

        public static bool ImportLegacyEnvironment(string root, out string message)
        {
            message = null;
            string storePath = StorePath(root);
            string envPath = Path.Combine(root, ".env");
            if (File.Exists(storePath) || !File.Exists(envPath)) return false;

            Dictionary<string, string> values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (string line in File.ReadAllLines(envPath, Encoding.UTF8))
            {
                string trimmed = line.Trim();
                if (trimmed.Length == 0 || trimmed.StartsWith("#")) continue;
                int eq = trimmed.IndexOf('=');
                if (eq <= 0) continue;
                values[trimmed.Substring(0, eq).Trim()] = trimmed.Substring(eq + 1).Trim();
            }
            string model, baseUrl, apiKey;
            if (!values.TryGetValue("UPSTREAM_MODEL", out model) || String.IsNullOrWhiteSpace(model)) return false;
            if (!values.TryGetValue("UPSTREAM_BASE_URL", out baseUrl) || String.IsNullOrWhiteSpace(baseUrl)) return false;
            if (!values.TryGetValue("UPSTREAM_API_KEY", out apiKey) || String.IsNullOrWhiteSpace(apiKey)) return false;
            if (apiKey == "replace-with-new-key") return false;

            string id = Guid.NewGuid().ToString("N");
            string modelId = model.Contains("/") ? model.Substring(model.IndexOf('/') + 1) : model;
            ModelProfile profile = new ModelProfile
            {
                id = id,
                name = "Imported model",
                base_url = baseUrl.TrimEnd('/'),
                api_key = apiKey,
                model_id = modelId,
                litellm_model = model
            };
            ModelStore store = EmptyStore();
            store.default_id = id;
            store.profiles.Add(profile);
            SaveStore(root, store);
            message = "已从旧版 .env 迁移模型配置：" + modelId;
            return true;
        }

        public static GatewayState ReadState(string root)
        {
            try
            {
                string path = Path.Combine(root, ".gateway", "state.json");
                if (!File.Exists(path)) return null;
                return Json.Deserialize<GatewayState>(File.ReadAllText(path, Encoding.UTF8));
            }
            catch { return null; }
        }

        public static bool AutostartEnabled()
        {
            string link = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Startup), "Codex Chat Gateway.lnk");
            return File.Exists(link);
        }

        public static bool HealthCheck()
        {
            try
            {
                HttpWebRequest request = (HttpWebRequest)WebRequest.Create("http://127.0.0.1:4000/health/liveliness");
                request.Timeout = 1200;
                request.ReadWriteTimeout = 1200;
                using (request.GetResponse()) { return true; }
            }
            catch { return false; }
        }

        public static List<string> FetchModels(string baseUrl, string apiKey, out string error)
        {
            error = null;
            List<string> ids = new List<string>();
            try
            {
                HttpWebRequest request = (HttpWebRequest)WebRequest.Create(baseUrl.TrimEnd('/') + "/models");
                request.Method = "GET";
                request.Accept = "application/json";
                request.Headers["Authorization"] = "Bearer " + apiKey;
                request.Timeout = 30000;
                request.ReadWriteTimeout = 30000;
                using (HttpWebResponse response = (HttpWebResponse)request.GetResponse())
                using (Stream stream = response.GetResponseStream())
                using (StreamReader reader = new StreamReader(stream, Encoding.UTF8))
                {
                    Dictionary<string, object> payload = Json.Deserialize<Dictionary<string, object>>(reader.ReadToEnd());
                    object data;
                    if (payload == null || !payload.TryGetValue("data", out data))
                    {
                        error = "接口返回中没有 data 字段。";
                        return ids;
                    }
                    ArrayList items = data as ArrayList;
                    if (items == null)
                    {
                        error = "接口返回格式不是标准模型列表。";
                        return ids;
                    }
                    foreach (object item in items)
                    {
                        Dictionary<string, object> entry = item as Dictionary<string, object>;
                        if (entry == null) continue;
                        object id;
                        if (entry.TryGetValue("id", out id) && id != null)
                        {
                            string text = id.ToString();
                            if (!String.IsNullOrWhiteSpace(text)) ids.Add(text);
                        }
                    }
                    ids = ids.Distinct().OrderBy(x => x, StringComparer.OrdinalIgnoreCase).ToList();
                    if (ids.Count == 0) error = "该接口没有返回任何模型。";
                    return ids;
                }
            }
            catch (Exception ex)
            {
                error = "获取失败：" + ex.Message;
                return ids;
            }
        }

        public static string MaskKey(string key)
        {
            if (String.IsNullOrEmpty(key)) return "";
            if (key.Length <= 8) return "********";
            return key.Substring(0, 3) + "..." + key.Substring(key.Length - 4);
        }
    }

    internal sealed class MainWindow : Window
    {
        private readonly string root;
        private readonly List<Button> actionButtons = new List<Button>();
        private readonly DispatcherTimer timer = new DispatcherTimer();
        private WinForms.NotifyIcon trayIcon;
        private ParticleField particles;

        private Ellipse statusDot;
        private TextBlock statusText;
        private TextBlock detailText;
        private TextBlock modelText;
        private TextBlock autostartTitle;
        private TextBlock autostartSub;
        private Button startButton;
        private Button stopButton;
        private Button restartButton;
        private DataGrid modelsGrid;
        private RichTextBox consoleBox;

        private bool busy;
        private bool allowExit;
        private bool trayHintShown;
        private bool? trayRunning;
        private bool? lastRunning;

        public MainWindow()
        {
            root = Store.FindProjectRoot(AppDomain.CurrentDomain.BaseDirectory);
            Title = "Codex Chat Gateway";
            Width = 1180;
            Height = 780;
            MinWidth = 1040;
            MinHeight = 700;
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            FontFamily = Theme.Ui;
            FontSize = 12;
            Foreground = Theme.PaperBrush;
            UseLayoutRounding = true;
            TextOptions.SetTextFormattingMode(this, TextFormattingMode.Display);
            Icon = LoadAppIcon();

            BuildShell();
            InitializeTray();

            timer.Interval = TimeSpan.FromSeconds(3);
            timer.Tick += async delegate { await RefreshStatusAsync(); };

            Loaded += OnLoaded;
            Closing += OnClosing;
            Closed += OnClosed;
            StateChanged += delegate { if (WindowState == WindowState.Minimized) HideToTray(); };
            PreviewMouseMove += delegate(object s, MouseEventArgs e) { particles.SetMouse(e.GetPosition(particles)); };
            MouseLeave += delegate { particles.SetMouse(new Point(-10000, -10000)); };
            IsVisibleChanged += delegate(object s, DependencyPropertyChangedEventArgs e)
            {
                if ((bool)e.NewValue) particles.Start(); else particles.Stop();
            };
        }

        private ImageSource LoadAppIcon()
        {
            try
            {
                SD.Icon icon = SD.Icon.ExtractAssociatedIcon(Process.GetCurrentProcess().MainModule.FileName);
                if (icon == null) return null;
                BitmapSource source = Imaging.CreateBitmapSourceFromHIcon(icon.Handle, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
                source.Freeze();
                return source;
            }
            catch { return null; }
        }

        private void BuildShell()
        {
            Grid host = new Grid();
            host.Background = new LinearGradientBrush(Theme.Ink, Theme.Deep, 90.0);
            particles = new ParticleField();
            host.Children.Add(particles);

            Grid shell = new Grid { Margin = new Thickness(26, 18, 26, 14) };
            shell.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            shell.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            shell.RowDefinitions.Add(new RowDefinition { Height = new GridLength(152) });

            FrameworkElement header = BuildHeader();
            Grid.SetRow(header, 0);
            shell.Children.Add(header);

            Grid body = new Grid { Margin = new Thickness(0, 14, 0, 14) };
            body.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(392) });
            body.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            FrameworkElement control = BuildControlCard();
            control.Margin = new Thickness(0, 0, 7, 0);
            Grid.SetColumn(control, 0);
            body.Children.Add(control);
            FrameworkElement models = BuildModelsCard();
            models.Margin = new Thickness(7, 0, 0, 0);
            Grid.SetColumn(models, 1);
            body.Children.Add(models);
            Grid.SetRow(body, 1);
            shell.Children.Add(body);

            FrameworkElement console = BuildConsole();
            Grid.SetRow(console, 2);
            shell.Children.Add(console);

            host.Children.Add(shell);
            Content = host;
        }

        private FrameworkElement BuildHeader()
        {
            Grid header = new Grid { Margin = new Thickness(2, 0, 2, 0) };
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

            Image logo = new Image { Width = 46, Height = 46, Source = Icon, VerticalAlignment = VerticalAlignment.Center };
            if (Icon == null) logo.Visibility = Visibility.Collapsed;
            header.Children.Add(logo);

            StackPanel texts = new StackPanel { Margin = new Thickness(14, 0, 0, 0), VerticalAlignment = VerticalAlignment.Center };
            Grid.SetColumn(texts, 1);
            texts.Children.Add(new TextBlock { Text = "LOCAL MODEL BRIDGE / WINDOWS", FontFamily = Theme.Mono, FontSize = 10, Foreground = Theme.MintBrush, FontWeight = FontWeights.Bold });
            texts.Children.Add(new TextBlock { Text = "Codex Chat Gateway", FontFamily = Theme.Ui, FontSize = 24, FontWeight = FontWeights.SemiBold, Foreground = Theme.PaperBrush, Margin = new Thickness(0, 2, 0, 3) });
            TextBlock sub = new TextBlock { FontFamily = Theme.Ui, FontSize = 10.5, Foreground = Theme.MutedBrush };
            sub.Inlines.Add(ReadVersion());
            sub.Inlines.Add("  ·  仅监听 127.0.0.1  ·  密钥只保存在本机");
            texts.Children.Add(sub);
            header.Children.Add(texts);

            Button github = Ui.Button("GitHub 仓库", ButtonKind.Ghost);
            github.VerticalAlignment = VerticalAlignment.Center;
            github.Click += delegate { OpenUrl("https://github.com/xuyuanzhang1122/codex-chat-gateway-windows"); };
            Grid.SetColumn(github, 2);
            header.Children.Add(github);
            return header;
        }

        private FrameworkElement BuildControlCard()
        {
            StackPanel panel = new StackPanel { Margin = new Thickness(20, 18, 20, 18) };
            panel.Children.Add(Ui.Section("网关状态 GATEWAY"));

            Grid statusRow = new Grid();
            statusRow.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            statusRow.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            statusDot = new Ellipse
            {
                Width = 12,
                Height = 12,
                Fill = Theme.DangerBrush,
                VerticalAlignment = VerticalAlignment.Center,
                Effect = new DropShadowEffect { Color = Theme.Danger, BlurRadius = 12, ShadowDepth = 0, Opacity = 0.95, RenderingBias = RenderingBias.Performance }
            };
            statusRow.Children.Add(statusDot);
            statusText = new TextBlock { Text = "正在检测", FontSize = 19, FontWeight = FontWeights.SemiBold, Foreground = Theme.PaperBrush, Margin = new Thickness(10, 0, 0, 0), VerticalAlignment = VerticalAlignment.Center };
            Grid.SetColumn(statusText, 1);
            statusRow.Children.Add(statusText);
            panel.Children.Add(statusRow);

            Grid endpointRow = new Grid { Margin = new Thickness(0, 10, 0, 0) };
            endpointRow.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            endpointRow.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            endpointRow.Children.Add(new TextBlock { Text = "http://127.0.0.1:4000/v1", FontFamily = Theme.Mono, FontSize = 11, Foreground = Theme.IceBrush, VerticalAlignment = VerticalAlignment.Center });
            Button copyEndpoint = Ui.Button("复制", ButtonKind.Ghost);
            copyEndpoint.FontSize = 10.5;
            copyEndpoint.Padding = new Thickness(9, 4, 9, 4);
            Grid.SetColumn(copyEndpoint, 1);
            copyEndpoint.Click += delegate { Clipboard.SetText("http://127.0.0.1:4000/v1"); Log("OK", "已复制接口地址 http://127.0.0.1:4000/v1"); };
            endpointRow.Children.Add(copyEndpoint);
            panel.Children.Add(endpointRow);

            detailText = new TextBlock { Text = "—", FontFamily = Theme.Mono, FontSize = 10.5, Foreground = Theme.MutedBrush, Margin = new Thickness(0, 9, 0, 0), TextTrimming = TextTrimming.CharacterEllipsis };
            panel.Children.Add(detailText);
            modelText = new TextBlock { Text = "默认模型：—", FontSize = 11.5, Foreground = Theme.PaperBrush, Margin = new Thickness(0, 6, 0, 0), TextTrimming = TextTrimming.CharacterEllipsis };
            panel.Children.Add(modelText);

            UniformGrid rowA = new UniformGrid { Columns = 3, Margin = new Thickness(0, 14, 0, 0) };
            startButton = Reg(Ui.Button("启动网关", ButtonKind.Primary));
            startButton.Margin = new Thickness(0, 0, 4, 0);
            startButton.Click += async delegate { await StartGatewayFromUiAsync(); };
            stopButton = Reg(Ui.Button("停止", ButtonKind.Ghost));
            stopButton.Margin = new Thickness(4, 0, 4, 0);
            stopButton.Click += async delegate { await RunActionsAsync("停止网关", false, "stop-background.ps1"); };
            restartButton = Reg(Ui.Button("重启", ButtonKind.Ghost));
            restartButton.Margin = new Thickness(4, 0, 0, 0);
            restartButton.Click += async delegate { await RestartGatewayFromUiAsync(); };
            rowA.Children.Add(startButton);
            rowA.Children.Add(stopButton);
            rowA.Children.Add(restartButton);
            panel.Children.Add(rowA);

            UniformGrid rowB = new UniformGrid { Columns = 3, Margin = new Thickness(0, 8, 0, 0) };
            Button check = Reg(Ui.Button("检查接口", ButtonKind.Ghost));
            check.Margin = new Thickness(0, 0, 4, 0);
            check.Click += async delegate { await RunActionsAsync("接口检查", false, "check.ps1"); };
            Button logs = Reg(Ui.Button("打开日志", ButtonKind.Ghost));
            logs.Margin = new Thickness(4, 0, 4, 0);
            logs.Click += delegate { OpenFolder(Path.Combine(root, "logs")); };
            Button openUi = Reg(Ui.Button("打开地址", ButtonKind.Ghost));
            openUi.Margin = new Thickness(4, 0, 0, 0);
            openUi.Click += delegate { OpenUrl("http://127.0.0.1:4000/ui"); };
            rowB.Children.Add(check);
            rowB.Children.Add(logs);
            rowB.Children.Add(openUi);
            panel.Children.Add(rowB);

            TextBlock clients = Ui.Section("客户端接入 CLIENTS");
            clients.Margin = new Thickness(0, 18, 0, 10);
            panel.Children.Add(clients);

            Button codex = Reg(Ui.WideButton("配置 Codex", "Responses API → 本地网关（先备份，保留 MCP）", ButtonKind.Ghost));
            codex.Click += async delegate { await RunActionsAsync("配置 Codex", false, "configure-codex.ps1"); };
            panel.Children.Add(codex);
            Button claude = Reg(Ui.WideButton("配置 Claude Desktop", "Code 模式 → 本地网关（独立 3P Profile）", ButtonKind.Ghost));
            claude.Margin = new Thickness(0, 8, 0, 0);
            claude.Click += async delegate { await RunActionsAsync("配置 Claude Desktop", false, "configure-claude-desktop.ps1"); };
            panel.Children.Add(claude);

            UniformGrid rowC = new UniformGrid { Columns = 2, Margin = new Thickness(0, 8, 0, 0) };
            Button restoreCodex = Reg(Ui.Button("恢复 Codex", ButtonKind.Ghost));
            restoreCodex.Margin = new Thickness(0, 0, 4, 0);
            restoreCodex.Click += async delegate { await RunActionsAsync("恢复 Codex 官方配置", false, "restore-codex.ps1"); };
            Button restoreClaude = Reg(Ui.Button("恢复 Claude", ButtonKind.Ghost));
            restoreClaude.Margin = new Thickness(4, 0, 0, 0);
            restoreClaude.Click += async delegate { await RunActionsAsync("恢复 Claude 官方配置", false, "restore-claude-desktop.ps1"); };
            rowC.Children.Add(restoreCodex);
            rowC.Children.Add(restoreClaude);
            panel.Children.Add(rowC);

            autostartButton = Reg(Ui.Button("", ButtonKind.Ghost));
            autostartButton.HorizontalContentAlignment = HorizontalAlignment.Left;
            autostartButton.Padding = new Thickness(14, 9, 14, 9);
            autostartButton.Margin = new Thickness(0, 8, 0, 0);
            StackPanel autoContent = new StackPanel();
            autostartTitle = new TextBlock { Foreground = Theme.PaperBrush, FontSize = 12.5, FontWeight = FontWeights.SemiBold };
            autostartSub = new TextBlock { Foreground = Theme.MutedBrush, FontSize = 10.5, Margin = new Thickness(0, 2, 0, 0) };
            autoContent.Children.Add(autostartTitle);
            autoContent.Children.Add(autostartSub);
            autostartButton.Content = autoContent;
            autostartButton.Click += async delegate
            {
                bool on = Store.AutostartEnabled();
                await RunActionsAsync(on ? "关闭登录自启" : "启用登录自启", false, on ? "disable-autostart.ps1" : "enable-autostart.ps1");
            };
            panel.Children.Add(autostartButton);

            ScrollViewer scroll = new ScrollViewer { Content = panel, VerticalScrollBarVisibility = ScrollBarVisibility.Auto, HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled };
            return Ui.Card(scroll);
        }

        private Button autostartButton;

        private FrameworkElement BuildModelsCard()
        {
            Grid grid = new Grid { Margin = new Thickness(20, 18, 20, 14) };
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            grid.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });

            Grid top = new Grid();
            top.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            top.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            TextBlock section = Ui.Section("模型配置 MODELS");
            top.Children.Add(section);
            Button add = Reg(Ui.Button("＋ 添加模型", ButtonKind.Primary));
            add.FontSize = 11;
            add.Padding = new Thickness(12, 5, 12, 5);
            add.Click += delegate { AddModel(); };
            Grid.SetColumn(add, 1);
            top.Children.Add(add);
            grid.Children.Add(top);

            TextBlock hint = new TextBlock { Text = "密钥仅保存在当前用户的 .gateway/models.json，不会写入任何客户端配置。", Foreground = Theme.MutedBrush, FontSize = 10.5, Margin = new Thickness(0, 2, 0, 10) };
            Grid.SetRow(hint, 1);
            grid.Children.Add(hint);

            modelsGrid = BuildGrid();
            Grid.SetRow(modelsGrid, 2);
            grid.Children.Add(modelsGrid);

            StackPanel actions = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 10, 0, 0) };
            Button makeDefault = Reg(Ui.Button("设为默认", ButtonKind.Ghost));
            makeDefault.Margin = new Thickness(0, 0, 8, 0);
            makeDefault.Click += delegate { SetDefaultModel(); };
            Button edit = Reg(Ui.Button("编辑", ButtonKind.Ghost));
            edit.Margin = new Thickness(0, 0, 8, 0);
            edit.Click += delegate { EditModel(); };
            Button remove = Reg(Ui.Button("删除", ButtonKind.Danger));
            remove.Margin = new Thickness(0, 0, 8, 0);
            remove.Click += delegate { DeleteModel(); };
            Button refresh = Reg(Ui.Button("刷新", ButtonKind.Ghost));
            refresh.Click += delegate { ReloadModels(); };
            actions.Children.Add(makeDefault);
            actions.Children.Add(edit);
            actions.Children.Add(remove);
            actions.Children.Add(refresh);
            Grid.SetRow(actions, 3);
            grid.Children.Add(actions);
            return Ui.Card(grid);
        }

        private DataGrid BuildGrid()
        {
            DataGrid grid = new DataGrid
            {
                AutoGenerateColumns = false,
                IsReadOnly = true,
                CanUserAddRows = false,
                CanUserDeleteRows = false,
                CanUserResizeRows = false,
                SelectionMode = DataGridSelectionMode.Single,
                SelectionUnit = DataGridSelectionUnit.FullRow,
                Background = Brushes.Transparent,
                BorderThickness = new Thickness(0),
                RowBackground = Brushes.Transparent,
                AlternatingRowBackground = Theme.BA(Theme.PanelSoft, 0x66),
                GridLinesVisibility = DataGridGridLinesVisibility.Horizontal,
                HorizontalGridLinesBrush = Theme.BA(Theme.CardLine, 0x99),
                RowHeight = 34,
                ColumnHeaderHeight = 32,
                Foreground = Theme.PaperBrush,
                FontSize = 11.5,
                HeadersVisibility = DataGridHeadersVisibility.Column,
                Cursor = Cursors.Hand
            };

            Style headerStyle = new Style(typeof(DataGridColumnHeader));
            headerStyle.Setters.Add(new Setter(Control.BackgroundProperty, Theme.B(Theme.PanelSoft)));
            headerStyle.Setters.Add(new Setter(Control.ForegroundProperty, Theme.MutedBrush));
            headerStyle.Setters.Add(new Setter(Control.FontWeightProperty, FontWeights.SemiBold));
            headerStyle.Setters.Add(new Setter(Control.FontSizeProperty, 10.5));
            headerStyle.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(10, 0, 4, 0)));
            headerStyle.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.LineBrush));
            headerStyle.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(0, 0, 1, 1)));
            headerStyle.Setters.Add(new Setter(Control.HorizontalContentAlignmentProperty, HorizontalAlignment.Left));
            grid.ColumnHeaderStyle = headerStyle;

            Style cellStyle = new Style(typeof(DataGridCell));
            cellStyle.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(10, 0, 4, 0)));
            cellStyle.Setters.Add(new Setter(DataGridCell.BorderThicknessProperty, new Thickness(0)));
            Trigger selected = new Trigger { Property = DataGridCell.IsSelectedProperty, Value = true };
            selected.Setters.Add(new Setter(DataGridCell.BackgroundProperty, Theme.B(Theme.Selection)));
            selected.Setters.Add(new Setter(DataGridCell.ForegroundProperty, Theme.PaperBrush));
            cellStyle.Triggers.Add(selected);
            grid.CellStyle = cellStyle;

            Style rowStyle = new Style(typeof(DataGridRow));
            DataTrigger isDefault = new DataTrigger { Binding = new System.Windows.Data.Binding("IsDefault"), Value = true };
            isDefault.Setters.Add(new Setter(DataGridRow.ForegroundProperty, Theme.MintBrush));
            rowStyle.Triggers.Add(isDefault);
            Trigger hoverRow = new Trigger { Property = DataGridRow.IsMouseOverProperty, Value = true };
            hoverRow.Setters.Add(new Setter(DataGridRow.BackgroundProperty, Theme.BA(Theme.Selection, 0x55)));
            rowStyle.Triggers.Add(hoverRow);
            grid.RowStyle = rowStyle;

            grid.Columns.Add(TextColumn("配置名称", "Name", 2.0));
            grid.Columns.Add(TextColumn("上游模型", "ModelId", 2.0));
            grid.Columns.Add(TextColumn("适配器", "Adapter", 1.7));
            grid.Columns.Add(TextColumn("API 地址", "BaseUrl", 2.5));
            grid.MouseDoubleClick += delegate { EditModel(); };
            return grid;
        }

        private static DataGridTextColumn TextColumn(string header, string path, double star)
        {
            Style elementStyle = new Style(typeof(TextBlock));
            elementStyle.Setters.Add(new Setter(TextBlock.VerticalAlignmentProperty, VerticalAlignment.Center));
            elementStyle.Setters.Add(new Setter(TextBlock.TextTrimmingProperty, TextTrimming.CharacterEllipsis));
            return new DataGridTextColumn
            {
                Header = header,
                Binding = new System.Windows.Data.Binding(path),
                Width = new DataGridLength(star, DataGridLengthUnitType.Star),
                ElementStyle = elementStyle
            };
        }

        private FrameworkElement BuildConsole()
        {
            Grid grid = new Grid { Margin = new Thickness(16, 10, 16, 12) };
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            grid.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

            DockPanel bar = new DockPanel { Margin = new Thickness(4, 0, 4, 6), LastChildFill = false };
            TextBlock label = new TextBlock { Text = "输出 OUTPUT", Foreground = Theme.MutedBrush, FontSize = 10.5, FontWeight = FontWeights.SemiBold, VerticalAlignment = VerticalAlignment.Center };
            DockPanel.SetDock(label, Dock.Left);
            bar.Children.Add(label);
            Button clear = Ui.Button("清空", ButtonKind.Ghost);
            clear.FontSize = 10.5;
            clear.Padding = new Thickness(9, 3, 9, 3);
            clear.Margin = new Thickness(6, 0, 0, 0);
            DockPanel.SetDock(clear, Dock.Right);
            clear.Click += delegate { consoleBox.Document.Blocks.Clear(); };
            bar.Children.Add(clear);
            Button copy = Ui.Button("复制", ButtonKind.Ghost);
            copy.FontSize = 10.5;
            copy.Padding = new Thickness(9, 3, 9, 3);
            DockPanel.SetDock(copy, Dock.Right);
            copy.Click += delegate
            {
                string text = new TextRange(consoleBox.Document.ContentStart, consoleBox.Document.ContentEnd).Text;
                if (!String.IsNullOrWhiteSpace(text)) Clipboard.SetText(text);
            };
            bar.Children.Add(copy);
            grid.Children.Add(bar);

            consoleBox = new RichTextBox
            {
                IsReadOnly = true,
                Background = Theme.FieldBrush,
                BorderThickness = new Thickness(0),
                FontFamily = Theme.Mono,
                FontSize = 11,
                Foreground = Theme.MutedBrush,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                Padding = new Thickness(8, 4, 8, 4),
                Document = new FlowDocument()
            };
            Grid.SetRow(consoleBox, 1);
            grid.Children.Add(consoleBox);
            return Ui.Card(grid);
        }

        private T Reg<T>(T button) where T : Button
        {
            actionButtons.Add(button);
            return button;
        }

        private async void OnLoaded(object sender, RoutedEventArgs e)
        {
            Log("INFO", "桌面控制台已就绪。项目目录：" + root);
            try
            {
                string migrateMessage;
                if (Store.ImportLegacyEnvironment(root, out migrateMessage)) Log("OK", migrateMessage);
            }
            catch (Exception ex) { Log("ERR", ".env 迁移失败：" + ex.Message); }
            ReloadModels();
            await RefreshStatusAsync();
            timer.Start();
        }

        private void OnClosing(object sender, System.ComponentModel.CancelEventArgs e)
        {
            if (allowExit)
            {
                if (trayIcon != null) trayIcon.Visible = false;
                return;
            }
            e.Cancel = true;
            HideToTray();
        }

        private void OnClosed(object sender, EventArgs e)
        {
            particles.Stop();
            if (trayIcon != null)
            {
                trayIcon.Visible = false;
                trayIcon.Dispose();
            }
        }

        private void HideToTray()
        {
            ShowInTaskbar = false;
            Hide();
            if (!trayHintShown)
            {
                trayHintShown = true;
                trayIcon.BalloonTipTitle = "Codex Chat Gateway 仍在后台";
                trayIcon.BalloonTipText = "双击托盘图标可恢复窗口；网关服务不会因关闭窗口而停止。";
                trayIcon.ShowBalloonTip(3000);
            }
        }

        public void ShowFromTray()
        {
            ShowInTaskbar = true;
            Show();
            WindowState = WindowState.Normal;
            Activate();
            Topmost = true;
            Topmost = false;
            Focus();
        }

        private void InitializeTray()
        {
            WinForms.ContextMenuStrip menu = new WinForms.ContextMenuStrip();
            WinForms.ToolStripMenuItem show = new WinForms.ToolStripMenuItem("显示桌面控制台");
            WinForms.ToolStripMenuItem start = new WinForms.ToolStripMenuItem("启动网关");
            WinForms.ToolStripMenuItem stop = new WinForms.ToolStripMenuItem("停止网关");
            WinForms.ToolStripMenuItem exit = new WinForms.ToolStripMenuItem("退出桌面控制台");
            show.Font = new SD.Font(show.Font, SD.FontStyle.Bold);
            show.Click += delegate { ShowFromTray(); };
            start.Click += async delegate { await StartGatewayFromUiAsync(); };
            stop.Click += async delegate { await RunActionsAsync("停止网关", false, "stop-background.ps1"); };
            exit.Click += delegate { allowExit = true; trayIcon.Visible = false; Close(); };
            menu.Items.AddRange(new WinForms.ToolStripItem[] { show, new WinForms.ToolStripSeparator(), start, stop, new WinForms.ToolStripSeparator(), exit });
            trayIcon = new WinForms.NotifyIcon { ContextMenuStrip = menu, Text = "Codex Chat Gateway", Visible = true };
            trayIcon.DoubleClick += delegate { ShowFromTray(); };
            UpdateTray(false);
        }

        private void UpdateTray(bool running)
        {
            if (trayRunning.HasValue && trayRunning.Value == running) return;
            trayRunning = running;
            SD.Icon previous = trayIcon.Icon;
            trayIcon.Icon = CreateTrayIcon(running ? SD.Color.FromArgb(0, 245, 212) : SD.Color.FromArgb(126, 139, 150));
            trayIcon.Text = running ? "Codex Chat Gateway — 运行中" : "Codex Chat Gateway — 已停止";
            if (previous != null) previous.Dispose();
        }

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        private static extern bool DestroyIcon(IntPtr handle);

        private static SD.Icon CreateTrayIcon(SD.Color color)
        {
            using (SD.Bitmap bitmap = new SD.Bitmap(32, 32))
            using (SD.Graphics graphics = SD.Graphics.FromImage(bitmap))
            using (SD.Pen pen = new SD.Pen(color, 4F))
            using (SD.Brush brush = new SD.SolidBrush(color))
            {
                graphics.SmoothingMode = SD.Drawing2D.SmoothingMode.AntiAlias;
                graphics.Clear(SD.Color.Transparent);
                pen.StartCap = SD.Drawing2D.LineCap.Round;
                pen.EndCap = SD.Drawing2D.LineCap.Round;
                pen.LineJoin = SD.Drawing2D.LineJoin.Round;
                graphics.DrawLine(pen, 16, 7, 7, 24);
                graphics.DrawLine(pen, 16, 7, 25, 24);
                graphics.DrawLine(pen, 7, 24, 25, 24);
                graphics.FillEllipse(brush, 12, 3, 8, 8);
                graphics.FillEllipse(brush, 3, 20, 8, 8);
                graphics.FillEllipse(brush, 21, 20, 8, 8);
                IntPtr handle = bitmap.GetHicon();
                try { return (SD.Icon)SD.Icon.FromHandle(handle).Clone(); }
                finally { DestroyIcon(handle); }
            }
        }

        private string ReadVersion()
        {
            try { return "v" + File.ReadAllText(Path.Combine(root, "VERSION")).Trim(); }
            catch { return "desktop"; }
        }

        private async Task RefreshStatusAsync()
        {
            bool running = await Task.Run(new Func<bool>(Store.HealthCheck));
            GatewayState state = running ? Store.ReadState(root) : null;
            if (!lastRunning.HasValue || lastRunning.Value != running)
            {
                AnimateDot(running);
                particles.Running = running;
                UpdateTray(running);
                lastRunning = running;
            }
            statusText.Text = running ? "运行中" : "已停止";
            statusText.Foreground = running ? Theme.MintBrush : Theme.PaperBrush;
            modelText.Text = "默认模型：" + GetDefaultModelName();
            if (running && state != null)
            {
                string uptime = FormatUptime(state.started_at);
                detailText.Text = "PID " + state.pid
                    + (uptime == null ? "" : " · 已运行 " + uptime)
                    + (String.IsNullOrEmpty(state.model) ? "" : " · " + state.model);
            }
            else if (running)
            {
                detailText.Text = "健康检查通过 · 无状态文件";
            }
            else
            {
                detailText.Text = "网关未在运行 · 启动后此处显示 PID 与运行时长";
            }
            startButton.IsEnabled = !running && !busy;
            stopButton.IsEnabled = running && !busy;
            restartButton.IsEnabled = running && !busy;
            RefreshAutostart();
        }

        private void AnimateDot(bool running)
        {
            statusDot.Fill = running ? Theme.MintBrush : Theme.DangerBrush;
            ((DropShadowEffect)statusDot.Effect).Color = running ? Theme.Mint : Theme.Danger;
            if (running)
            {
                DoubleAnimation pulse = new DoubleAnimation(1.0, 0.4, new Duration(TimeSpan.FromSeconds(1.3)))
                {
                    AutoReverse = true,
                    RepeatBehavior = RepeatBehavior.Forever
                };
                statusDot.BeginAnimation(UIElement.OpacityProperty, pulse);
            }
            else
            {
                statusDot.BeginAnimation(UIElement.OpacityProperty, null);
                statusDot.Opacity = 1.0;
            }
        }

        private void RefreshAutostart()
        {
            bool on = Store.AutostartEnabled();
            autostartTitle.Text = on ? "登录自启 · 已开启" : "登录自启 · 已关闭";
            autostartSub.Text = on ? "点击关闭登录自启" : "点击开启：登录 Windows 后自动启动网关";
        }

        private static string FormatUptime(string startedAt)
        {
            DateTimeOffset started;
            if (!DateTimeOffset.TryParse(startedAt, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind, out started)) return null;
            TimeSpan span = DateTimeOffset.Now - started;
            if (span.TotalSeconds < 0) return null;
            if (span.TotalMinutes < 1) return (int)span.TotalSeconds + " 秒";
            if (span.TotalHours < 1) return (int)span.TotalMinutes + " 分钟";
            if (span.TotalDays < 1) return (int)span.TotalHours + " 小时 " + span.Minutes + " 分";
            return (int)span.TotalDays + " 天 " + span.Hours + " 小时";
        }

        private void SetButtonsEnabled(bool enabled)
        {
            foreach (Button button in actionButtons) button.IsEnabled = enabled;
        }

        private bool HasDefaultModel()
        {
            try
            {
                ModelStore store = Store.ReadStore(root);
                return store.profiles.Any(x => x.id == store.default_id);
            }
            catch (Exception ex)
            {
                Log("ERR", "无法读取模型配置：" + ex.Message);
                return false;
            }
        }

        private async Task StartGatewayFromUiAsync()
        {
            if (!HasDefaultModel())
            {
                Log("DIM", "启动前需要先添加一个默认模型，正在打开桌面模型配置。");
                if (!AddModel())
                {
                    Log("ERR", "尚未配置默认模型，已取消启动。");
                    return;
                }
            }
            await RunActionsAsync("启动网关", false, "start-background.ps1");
        }

        private async Task RestartGatewayFromUiAsync()
        {
            if (!HasDefaultModel())
            {
                Log("DIM", "重启前需要先添加一个默认模型，正在打开桌面模型配置。");
                if (!AddModel())
                {
                    Log("ERR", "尚未配置默认模型，已取消重启；当前网关未被停止。");
                    return;
                }
            }
            await RunActionsAsync("重启网关", true, "stop-background.ps1", "start-background.ps1");
        }

        private async Task RunActionsAsync(string label, bool ignoreStopError, params string[] scripts)
        {
            if (busy) return;
            busy = true;
            timer.Stop();
            SetButtonsEnabled(false);
            Log("INFO", "▶ " + label);
            bool failed = false;
            try
            {
                foreach (string script in scripts)
                {
                    int code = await RunScriptAsync(script);
                    bool tolerate = ignoreStopError && script.StartsWith("stop", StringComparison.OrdinalIgnoreCase);
                    if (code != 0 && !tolerate)
                    {
                        Log("ERR", label + " 失败（退出码 " + code + "）。");
                        failed = true;
                        break;
                    }
                    if (code != 0 && tolerate) Log("DIM", "（忽略停止阶段的退出码 " + code + "）");
                }
                if (!failed) Log("OK", label + " 完成。");
            }
            finally
            {
                busy = false;
                SetButtonsEnabled(true);
            }
            ReloadModels();
            await RefreshStatusAsync();
            timer.Start();
        }

        private Task<int> RunScriptAsync(string script)
        {
            string path = Path.Combine(root, "scripts", script);
            if (!File.Exists(path))
            {
                Log("ERR", "缺少脚本：" + path);
                return Task.FromResult(1);
            }
            TaskCompletionSource<int> completion = new TaskCompletionSource<int>();
            string arguments = "-NoLogo -NoProfile -ExecutionPolicy Bypass -File \"" + path + "\"";
            if (script.Equals("start-background.ps1", StringComparison.OrdinalIgnoreCase))
                arguments += " -NonInteractive";
            ProcessStartInfo info = new ProcessStartInfo("powershell.exe", arguments)
            {
                WorkingDirectory = root,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8
            };
            Process process = new Process { StartInfo = info, EnableRaisingEvents = true };
            process.OutputDataReceived += delegate(object s, DataReceivedEventArgs e) { if (e.Data != null) LogOnUi("DIM", e.Data); };
            process.ErrorDataReceived += delegate(object s, DataReceivedEventArgs e) { if (e.Data != null) LogOnUi("ERR", e.Data); };
            process.Exited += delegate { completion.TrySetResult(process.ExitCode); process.Dispose(); };
            try
            {
                process.Start();
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
            }
            catch (Exception ex)
            {
                Log("ERR", "无法启动 PowerShell：" + ex.Message);
                completion.TrySetResult(1);
            }
            return completion.Task;
        }

        private void LogOnUi(string level, string message)
        {
            if (consoleBox == null) return;
            consoleBox.Dispatcher.BeginInvoke(new Action(delegate { Log(level, message); }));
        }

        private void Log(string level, string message)
        {
            if (consoleBox == null || String.IsNullOrEmpty(message)) return;
            Brush levelBrush = Theme.IceBrush;
            if (level == "OK") levelBrush = Theme.MintBrush;
            else if (level == "ERR") levelBrush = Theme.DangerBrush;
            else if (level == "DIM") levelBrush = Theme.MutedBrush;
            Paragraph paragraph = new Paragraph { Margin = new Thickness(0, 1, 0, 1) };
            paragraph.Inlines.Add(new Run(level.PadRight(4)) { Foreground = levelBrush, FontWeight = FontWeights.SemiBold });
            paragraph.Inlines.Add(new Run("  " + message) { Foreground = Theme.BA(Theme.Paper, 0xD8) });
            consoleBox.Document.Blocks.Add(paragraph);
            while (consoleBox.Document.Blocks.Count > 500)
                consoleBox.Document.Blocks.Remove(consoleBox.Document.Blocks.FirstBlock);
            consoleBox.ScrollToEnd();
        }

        private void ReloadModels()
        {
            ModelStore store;
            try { store = Store.ReadStore(root); }
            catch (Exception ex)
            {
                Log("ERR", "无法读取模型配置：" + ex.Message);
                store = Store.EmptyStore();
            }
            List<ModelRow> rows = new List<ModelRow>();
            foreach (ModelProfile p in store.profiles)
            {
                bool isDefault = p.id == store.default_id;
                rows.Add(new ModelRow
                {
                    Id = p.id,
                    Name = (isDefault ? "● " : "") + p.name,
                    ModelId = p.model_id,
                    Adapter = p.litellm_model,
                    BaseUrl = p.base_url,
                    IsDefault = isDefault
                });
            }
            modelsGrid.ItemsSource = rows;
        }

        private string GetDefaultModelName()
        {
            try
            {
                ModelStore store = Store.ReadStore(root);
                ModelProfile profile = store.profiles.FirstOrDefault(x => x.id == store.default_id);
                return profile == null ? "未配置" : profile.name;
            }
            catch { return "未配置"; }
        }

        private string SelectedId()
        {
            ModelRow row = modelsGrid.SelectedItem as ModelRow;
            return row == null ? null : row.Id;
        }

        private bool AddModel()
        {
            try
            {
                ModelDialog dialog = new ModelDialog(null) { Owner = this };
                if (dialog.ShowDialog() != true) return false;
                ModelStore store = Store.ReadStore(root);
                dialog.Profile.id = Guid.NewGuid().ToString("N");
                store.profiles.Add(dialog.Profile);
                if (String.IsNullOrEmpty(store.default_id)) store.default_id = dialog.Profile.id;
                Store.SaveStore(root, store);
                ReloadModels();
                Log("OK", "已保存模型配置：" + dialog.Profile.name);
                return true;
            }
            catch (Exception ex)
            {
                Log("ERR", "无法打开或保存模型配置：" + ex.Message);
                return false;
            }
        }

        private void EditModel()
        {
            try
            {
                string id = SelectedId();
                if (id == null) { Log("DIM", "请先在列表中选择一个模型。"); return; }
                ModelStore store = Store.ReadStore(root);
                ModelProfile profile = store.profiles.FirstOrDefault(x => x.id == id);
                if (profile == null) return;
                ModelDialog dialog = new ModelDialog(profile) { Owner = this };
                if (dialog.ShowDialog() != true) return;
                int index = store.profiles.FindIndex(x => x.id == id);
                dialog.Profile.id = id;
                store.profiles[index] = dialog.Profile;
                Store.SaveStore(root, store);
                ReloadModels();
                Log("OK", "已更新模型配置：" + dialog.Profile.name);
            }
            catch (Exception ex)
            {
                Log("ERR", "无法打开或保存模型配置：" + ex.Message);
            }
        }

        private void SetDefaultModel()
        {
            string id = SelectedId();
            if (id == null) { Log("DIM", "请先在列表中选择一个模型。"); return; }
            ModelStore store = Store.ReadStore(root);
            store.default_id = id;
            Store.SaveStore(root, store);
            ReloadModels();
            Log("OK", "已切换默认模型；点「重启」后生效。");
        }

        private void DeleteModel()
        {
            string id = SelectedId();
            if (id == null) { Log("DIM", "请先在列表中选择一个模型。"); return; }
            ModelStore store = Store.ReadStore(root);
            ModelProfile profile = store.profiles.FirstOrDefault(x => x.id == id);
            if (profile == null) return;
            if (MessageBox.Show(this, "删除模型配置“" + profile.name + "”？", "确认删除", MessageBoxButton.OKCancel, MessageBoxImage.Warning) != MessageBoxResult.OK) return;
            store.profiles.RemoveAll(x => x.id == id);
            if (store.default_id == id) store.default_id = store.profiles.Count == 0 ? "" : store.profiles[0].id;
            Store.SaveStore(root, store);
            ReloadModels();
            Log("OK", "已删除模型配置：" + profile.name);
        }

        private static void OpenFolder(string path)
        {
            Directory.CreateDirectory(path);
            Process.Start(new ProcessStartInfo("explorer.exe", "\"" + path + "\"") { UseShellExecute = true });
        }

        private static void OpenUrl(string url)
        {
            Process.Start(new ProcessStartInfo(url) { UseShellExecute = true });
        }
    }

    internal sealed class ModelDialog : Window
    {
        private readonly TextBox nameBox;
        private readonly TextBox urlBox;
        private readonly PasswordBox keyBox;
        private readonly TextBox plainKeyBox;
        private readonly TextBox modelBox;
        private readonly TextBlock statusText;
        private readonly Button fetchButton;

        public ModelProfile Profile { get; private set; }

        public ModelDialog(ModelProfile source)
        {
            Title = source == null ? "添加模型" : "编辑模型";
            Width = 520;
            Height = 596;
            ResizeMode = ResizeMode.NoResize;
            WindowStartupLocation = WindowStartupLocation.CenterOwner;
            Background = Theme.B(Theme.Deep);
            Foreground = Theme.PaperBrush;
            FontFamily = Theme.Ui;
            FontSize = 12;

            StackPanel panel = new StackPanel { Margin = new Thickness(26, 22, 26, 20) };
            panel.Children.Add(new TextBlock { Text = Title, FontSize = 18, FontWeight = FontWeights.SemiBold, Foreground = Theme.PaperBrush });

            panel.Children.Add(FieldLabel("配置名称", true));
            nameBox = Ui.Input();
            panel.Children.Add(nameBox);

            panel.Children.Add(FieldLabel("API Base URL（通常以 /v1 结尾）", false));
            urlBox = Ui.Input();
            panel.Children.Add(urlBox);

            panel.Children.Add(FieldLabel("API Key", false));
            Grid keyRow = new Grid();
            keyRow.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            keyRow.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            keyBox = Ui.PasswordInput();
            keyRow.Children.Add(keyBox);
            plainKeyBox = Ui.Input();
            plainKeyBox.Visibility = Visibility.Collapsed;
            keyRow.Children.Add(plainKeyBox);
            CheckBox showKey = new CheckBox { Content = "显示", Foreground = Theme.MutedBrush, VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(10, 0, 0, 0) };
            Grid.SetColumn(showKey, 1);
            showKey.Checked += delegate { plainKeyBox.Text = keyBox.Password; keyBox.Visibility = Visibility.Collapsed; plainKeyBox.Visibility = Visibility.Visible; };
            showKey.Unchecked += delegate { keyBox.Password = plainKeyBox.Text; plainKeyBox.Visibility = Visibility.Collapsed; keyBox.Visibility = Visibility.Visible; };
            keyRow.Children.Add(showKey);
            panel.Children.Add(keyRow);

            panel.Children.Add(FieldLabel("模型 ID", false));
            Grid modelRow = new Grid();
            modelRow.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            modelRow.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            modelBox = Ui.Input();
            modelRow.Children.Add(modelBox);
            fetchButton = Ui.Button("在线获取", ButtonKind.Ghost);
            fetchButton.FontSize = 11;
            fetchButton.Margin = new Thickness(8, 0, 0, 0);
            fetchButton.VerticalAlignment = VerticalAlignment.Center;
            fetchButton.Click += async delegate { await FetchModelsAsync(); };
            Grid.SetColumn(fetchButton, 1);
            modelRow.Children.Add(fetchButton);
            panel.Children.Add(modelRow);

            statusText = new TextBlock { Foreground = Theme.MutedBrush, FontSize = 10.5, TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 12, 0, 0), MinHeight = 16 };
            panel.Children.Add(statusText);

            StackPanel buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 16, 0, 0) };
            Button cancel = Ui.Button("取消", ButtonKind.Ghost);
            cancel.MinWidth = 92;
            cancel.Margin = new Thickness(0, 0, 10, 0);
            cancel.Click += delegate { DialogResult = false; };
            buttons.Children.Add(cancel);
            Button save = Ui.Button("保存配置", ButtonKind.Primary);
            save.MinWidth = 120;
            save.Click += delegate { ValidateAndSave(); };
            buttons.Children.Add(save);
            panel.Children.Add(buttons);

            Content = panel;

            if (source != null)
            {
                nameBox.Text = source.name;
                urlBox.Text = source.base_url;
                keyBox.Password = source.api_key ?? "";
                modelBox.Text = source.model_id;
            }
        }

        private TextBlock FieldLabel(string text, bool first)
        {
            return new TextBlock
            {
                Text = text,
                Foreground = Theme.MutedBrush,
                FontSize = 10.5,
                FontWeight = FontWeights.SemiBold,
                Margin = new Thickness(0, first ? 18 : 14, 0, 6)
            };
        }

        private string CurrentKey()
        {
            return plainKeyBox.Visibility == Visibility.Visible ? plainKeyBox.Text : keyBox.Password;
        }

        private void SetStatus(string text, Brush brush)
        {
            statusText.Text = text;
            statusText.Foreground = brush;
        }

        private async Task FetchModelsAsync()
        {
            string baseUrl = urlBox.Text.Trim().TrimEnd('/');
            Uri parsed;
            if (!Uri.TryCreate(baseUrl, UriKind.Absolute, out parsed) || (parsed.Scheme != "http" && parsed.Scheme != "https"))
            {
                SetStatus("请先填写有效的 HTTP(S) API 地址。", Theme.DangerBrush);
                return;
            }
            string key = CurrentKey();
            if (String.IsNullOrWhiteSpace(key))
            {
                SetStatus("请先填写 API Key。", Theme.DangerBrush);
                return;
            }
            fetchButton.IsEnabled = false;
            SetStatus("正在获取模型列表…", Theme.MutedBrush);
            string error = null;
            List<string> ids = null;
            try
            {
                await Task.Run(delegate
                {
                    string err;
                    List<string> result = Store.FetchModels(baseUrl, key, out err);
                    error = err;
                    ids = result;
                });
            }
            finally { fetchButton.IsEnabled = true; }
            if (ids == null || ids.Count == 0)
            {
                SetStatus((error ?? "未获取到模型。") + " 可改用手动输入模型 ID。", Theme.DangerBrush);
                return;
            }
            SetStatus("已获取 " + ids.Count + " 个模型，请在列表中选择。", Theme.MintBrush);
            ModelPickerWindow picker = new ModelPickerWindow(ids) { Owner = this };
            if (picker.ShowDialog() == true && picker.Selected != null)
                modelBox.Text = picker.Selected;
        }

        private void ValidateAndSave()
        {
            string cleanUrl = urlBox.Text.Trim().TrimEnd('/');
            Uri parsed;
            string key = CurrentKey();
            string modelId = modelBox.Text.Trim();
            if (String.IsNullOrWhiteSpace(nameBox.Text))
            {
                SetStatus("请填写配置名称。", Theme.DangerBrush);
                return;
            }
            if (!Uri.TryCreate(cleanUrl, UriKind.Absolute, out parsed) || (parsed.Scheme != "http" && parsed.Scheme != "https"))
            {
                SetStatus("请填写有效的 HTTP(S) API 地址。", Theme.DangerBrush);
                return;
            }
            if (String.IsNullOrWhiteSpace(key))
            {
                SetStatus("请填写 API Key。", Theme.DangerBrush);
                return;
            }
            if (String.IsNullOrWhiteSpace(modelId))
            {
                SetStatus("请填写模型 ID，或点「在线获取」从列表选择。", Theme.DangerBrush);
                return;
            }
            Profile = new ModelProfile
            {
                name = nameBox.Text.Trim(),
                base_url = cleanUrl,
                api_key = key,
                model_id = modelId,
                litellm_model = Store.LiteLLMModel(cleanUrl, modelId)
            };
            DialogResult = true;
        }
    }

    internal sealed class ModelPickerWindow : Window
    {
        private readonly ListBox list;
        private readonly TextBox search;
        private readonly Button okButton;

        public string Selected { get; private set; }

        public ModelPickerWindow(List<string> ids)
        {
            Title = "选择模型（" + ids.Count + "）";
            Width = 480;
            Height = 540;
            WindowStartupLocation = WindowStartupLocation.CenterOwner;
            Background = Theme.B(Theme.Deep);
            Foreground = Theme.PaperBrush;
            FontFamily = Theme.Ui;
            FontSize = 12;
            SizeToContent = SizeToContent.Manual;

            Grid grid = new Grid { Margin = new Thickness(20, 18, 20, 16) };
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            grid.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });

            search = Ui.Input();
            search.TextChanged += delegate { ApplyFilter(ids); };
            grid.Children.Add(search);

            list = new ListBox
            {
                Margin = new Thickness(0, 10, 0, 10),
                Background = Theme.FieldBrush,
                BorderBrush = Theme.LineBrush,
                BorderThickness = new Thickness(1),
                FontFamily = Theme.Mono,
                FontSize = 11.5,
                ItemsSource = ids
            };
            Style itemStyle = new Style(typeof(ListBoxItem));
            itemStyle.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(10, 6, 10, 6)));
            itemStyle.Setters.Add(new Setter(Control.ForegroundProperty, Theme.PaperBrush));
            Trigger hover = new Trigger { Property = ListBoxItem.IsMouseOverProperty, Value = true };
            hover.Setters.Add(new Setter(ListBoxItem.BackgroundProperty, Theme.BA(Theme.Selection, 0x66)));
            itemStyle.Triggers.Add(hover);
            Trigger selectedTrigger = new Trigger { Property = ListBoxItem.IsSelectedProperty, Value = true };
            selectedTrigger.Setters.Add(new Setter(ListBoxItem.BackgroundProperty, Theme.B(Theme.Selection)));
            selectedTrigger.Setters.Add(new Setter(ListBoxItem.ForegroundProperty, Theme.MintBrush));
            itemStyle.Triggers.Add(selectedTrigger);
            list.ItemContainerStyle = itemStyle;
            list.SelectionChanged += delegate { okButton.IsEnabled = list.SelectedItem != null; };
            list.MouseDoubleClick += delegate { Commit(); };
            Grid.SetRow(list, 1);
            grid.Children.Add(list);

            StackPanel buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
            Button cancel = Ui.Button("取消", ButtonKind.Ghost);
            cancel.MinWidth = 92;
            cancel.Margin = new Thickness(0, 0, 10, 0);
            cancel.Click += delegate { DialogResult = false; };
            buttons.Children.Add(cancel);
            okButton = Ui.Button("使用此模型", ButtonKind.Primary);
            okButton.MinWidth = 110;
            okButton.IsEnabled = false;
            okButton.Click += delegate { Commit(); };
            buttons.Children.Add(okButton);
            Grid.SetRow(buttons, 2);
            grid.Children.Add(buttons);

            Content = grid;
            Loaded += delegate { search.Focus(); };
        }

        private void ApplyFilter(List<string> ids)
        {
            string needle = search.Text.Trim();
            ICollectionView view = CollectionViewSource.GetDefaultView(list.ItemsSource);
            if (needle.Length == 0) view.Filter = null;
            else view.Filter = delegate(object item) { return ((string)item).IndexOf(needle, StringComparison.OrdinalIgnoreCase) >= 0; };
        }

        private void Commit()
        {
            if (list.SelectedItem == null) return;
            Selected = (string)list.SelectedItem;
            DialogResult = true;
        }
    }

    internal static class SelfTest
    {
        [DllImport("kernel32.dll")]
        private static extern bool AttachConsole(int dwProcessId);

        public static int Run()
        {
            AttachConsole(-1);
            List<string> failures = new List<string>();
            Action<string, bool> check = delegate(string name, bool ok)
            {
                Console.WriteLine((ok ? "OK    " : "FAIL  ") + name);
                if (!ok) failures.Add(name);
            };

            string root = Store.FindProjectRoot(AppDomain.CurrentDomain.BaseDirectory);
            Console.WriteLine("root: " + root);

            check("litellm deepseek", Store.LiteLLMModel("https://api.deepseek.com/v1", "deepseek-chat") == "deepseek/deepseek-chat");
            check("litellm openai", Store.LiteLLMModel("https://api.moonshot.cn/v1", "kimi-k2") == "openai/kimi-k2");
            check("litellm keep prefix", Store.LiteLLMModel("https://api.deepseek.com/v1", "deepseek/deepseek-chat") == "deepseek/deepseek-chat");
            check("mask key", Store.MaskKey("sk-1234567890abcd") == "sk-...abcd");

            bool inputTemplatesOk = true;
            try
            {
                TextBox input = Ui.Input();
                PasswordBox password = Ui.PasswordInput();
                input.Measure(new Size(320, 42));
                password.Measure(new Size(320, 42));
                inputTemplatesOk = input.Template.TargetType == typeof(TextBox)
                    && password.Template.TargetType == typeof(PasswordBox);
            }
            catch (Exception ex)
            {
                Console.WriteLine("FAIL  input templates exception: " + ex.Message);
                inputTemplatesOk = false;
            }
            check("input templates", inputTemplatesOk);

            string storePath = Store.StorePath(root);
            string envPath = Path.Combine(root, ".env");
            byte[] storeBackup = File.Exists(storePath) ? File.ReadAllBytes(storePath) : null;
            byte[] envBackup = File.Exists(envPath) ? File.ReadAllBytes(envPath) : null;
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(storePath));
                if (File.Exists(storePath)) File.Delete(storePath);
                ModelStore store = Store.EmptyStore();
                store.profiles.Add(new ModelProfile { id = "a1", name = "T1", base_url = "https://x/v1", api_key = "k", model_id = "m1", litellm_model = "openai/m1" });
                store.profiles.Add(new ModelProfile { id = "b2", name = "T2", base_url = "https://y/v1", api_key = "k2", model_id = "m2", litellm_model = "openai/m2" });
                store.default_id = "b2";
                Store.SaveStore(root, store);
                ModelStore loaded = Store.ReadStore(root);
                check("store roundtrip", loaded.profiles.Count == 2 && loaded.default_id == "b2" && loaded.profiles[1].model_id == "m2");

                File.Delete(storePath);
                File.WriteAllText(envPath, "UPSTREAM_MODEL=deepseek/deepseek-chat\nUPSTREAM_BASE_URL=https://api.deepseek.com/v1\nUPSTREAM_API_KEY=sk-selftest-fake\n", new UTF8Encoding(false));
                string migrateMessage;
                bool migrated = Store.ImportLegacyEnvironment(root, out migrateMessage);
                check("env migrate", migrated);
                ModelStore migratedStore = Store.ReadStore(root);
                check("env migrate content", migratedStore.profiles.Count == 1
                    && migratedStore.profiles[0].model_id == "deepseek-chat"
                    && migratedStore.profiles[0].litellm_model == "deepseek/deepseek-chat"
                    && migratedStore.default_id == migratedStore.profiles[0].id);
                string skipMessage;
                check("env migrate skip when store exists", !Store.ImportLegacyEnvironment(root, out skipMessage));
            }
            catch (Exception ex)
            {
                Console.WriteLine("FAIL  exception: " + ex.Message);
                failures.Add("exception");
            }
            finally
            {
                if (storeBackup != null) File.WriteAllBytes(storePath, storeBackup);
                else if (File.Exists(storePath)) File.Delete(storePath);
                if (envBackup != null) File.WriteAllBytes(envPath, envBackup);
                else if (File.Exists(envPath)) File.Delete(envPath);
            }

            bool autostartOk = true;
            try { Store.AutostartEnabled(); } catch { autostartOk = false; }
            check("autostart detect no-throw", autostartOk);

            bool healthOk = true;
            try { Store.HealthCheck(); } catch { healthOk = false; }
            check("health check no-throw", healthOk);

            Console.WriteLine(failures.Count == 0 ? "SELFTEST PASS" : "SELFTEST FAIL (" + failures.Count + ")");
            return failures.Count == 0 ? 0 : 1;
        }
    }
}
