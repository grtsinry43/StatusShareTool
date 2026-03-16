namespace StatusShare.WindowsApp;

public partial class MainWindow : global::Wpf.Ui.Controls.FluentWindow
{
    public MainWindow()
    {
        InitializeComponent();
        DataContext = new ViewModels.MainWindowViewModel();
    }
}

