namespace StatusShare.Wpf;

public partial class MainWindow : Wpf.Ui.Controls.FluentWindow
{
    public MainWindow()
    {
        InitializeComponent();
        DataContext = new ViewModels.MainWindowViewModel();
    }
}

